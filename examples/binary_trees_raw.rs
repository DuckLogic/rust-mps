#![feature(
arbitrary_self_types, // Unfortunately this is required for methods on Gc refs
)]
//! An implementation of the [binary trees benchmark](https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/binarytrees.html#binarytrees)
//! that uses very low-level bindings to the MPS.
//!
//! This doesn't make use of any of the abstractions in zerogc. It should be equivalent
//! to C level usage of the MPS.
use slog::{Logger, Drain, o};
use std::cell::Cell;
use mps::format::{RawFormatMethods, ScanState, ObjectFormat};
use std::ffi::{c_void, CStr};
use mps::arena::{VirtualMemoryArenaClass, Arena};
use mps::pools::Pool;
use mps::pools::mark_sweep::AutoMarkSweep;
use mps::MpsError;
use mps::alloc::AllocationPoint;
use std::alloc::Layout;

#[repr(C)]
struct Tree<'gc> {
    // NOTE: This is horribly unsafe
    children: Cell<Option<(&'gc Tree<'gc>, &'gc Tree<'gc>)>>,
}
/// The special object format we use
enum TreeObject {
    Forwarding {
        new: *mut TreeObject,
        size: usize
    },
    Tree(Tree<'static>),
    Padding {
        size: usize
    }
}
impl TreeObject {
    fn size(&self) -> usize {
        let res = match *self {
            TreeObject::Forwarding { size, .. } => size,
            TreeObject::Tree(Tree { .. }) => Layout::new::<TreeObject>().pad_to_align().size(),
            TreeObject::Padding { size } => size,
        };
        debug_assert!(res >= std::mem::size_of::<TreeObject>());
        res
    }
}
unsafe impl RawFormatMethods for TreeObject {
    type Obj = TreeObject;
    const ALIGNMENT: usize = std::mem::align_of::<Self>();

    unsafe extern fn class_ptr(obj: *mut Self::Obj) -> *mut c_void {
        match *obj {
            TreeObject::Forwarding { .. } | TreeObject::Padding { .. } => std::ptr::null_mut(),
            TreeObject::Tree(_) => CStr::from_bytes_with_nul(b"Tree\0").unwrap().as_ptr() as *mut c_void
        }
    }

    unsafe extern fn forward(old: *mut Self::Obj, new: *mut Self::Obj) {
        old.write(TreeObject::Forwarding {
            new, size: (*old).size()
        })
    }

    unsafe extern fn is_forwarded(old: *mut Self::Obj) -> *mut Self::Obj {
        match old.read() {
            TreeObject::Forwarding { new, .. } => new,
            _ => std::ptr::null_mut()
        }
    }

    unsafe extern fn pad(addr: *mut Self::Obj, size: usize) {
        addr.write(TreeObject::Padding {
            size: size
        })
    }

    unsafe extern fn scan(mut state: ScanState, mut base: *mut Self::Obj, limit: *mut Self::Obj) -> i32 {
        state.fix_with(|state| {
            while base < limit {
                let obj: *mut TreeObject = base;
                match *obj {
                    TreeObject::Forwarding { ref mut new, size: _ } => {
                        // Forwarding objects must be scanned
                        state.fix(new)?;
                    },
                    TreeObject::Tree(Tree { ref mut children }) => {
                        if let Some((left, right)) = children.get_mut() {
                            state.fix(&mut *(left as *mut _ as *mut *mut Tree))?;
                            state.fix(&mut *(right as *mut _ as *mut *mut Tree))?;
                        }
                    }
                    TreeObject::Padding { size: _ } => {},
                }
                base = base.add(1);
            }
            Ok(())
        })
    }

    unsafe extern fn skip(addr: *mut Self::Obj) -> *mut Self::Obj {
        addr.add(addr.read().size())
    }
}

pub struct RawMpsCollector<'arena> {
    arena: &'arena Arena,
    allocation_point: AllocationPoint,
}

fn item_check(tree: &Tree) -> i32 {
    if let Some((left, right)) = tree.children.get() {
        1 + item_check(left) + item_check(right)
    } else {
        1
    }
}

/// Create a bottom up binary tree
///
/// ## Safety
/// This is unsafe, because it trusts the specified garbage collector to work properly.
unsafe fn bottom_up_tree<'gc>(collector: &'gc RawMpsCollector, depth: i32) -> Result<&'gc Tree<'gc>, MpsError> {
    let tree = &*collector.allocation_point.alloc_with(|ptr: *mut TreeObject| {
        ptr.write(TreeObject::Tree(Tree { children: Cell::new(None) }));
    })?;
    let tree = match tree {
        TreeObject::Tree(ref tree) => std::mem::transmute::<&Tree<'_>, &'gc Tree<'gc>>(tree),
        _ => unreachable!()
    };
    if depth > 0 {
        let right = bottom_up_tree(collector, depth - 1)?;
        let left = bottom_up_tree(collector, depth - 1)?;
        tree.children.set(Some((left, right)));
    }
    Ok(tree)
}

fn inner(
    gc: &RawMpsCollector,
    depth: i32, iterations: u32
) -> Result<String, MpsError> {
    let chk: i32 = (0 .. iterations).into_iter().map(|_| {
        let a = unsafe { bottom_up_tree(&gc, depth)? };
        Ok(item_check(&a))
    }).try_fold(0, |a, b| Ok(a + b?))?;
    Ok(format!("{}\t trees of depth {}\t check: {}", iterations, depth, chk))
}

enum PoolType {
    MarkSweep,
}
impl PoolType {
    fn create<'a>(&self, arena: &'a Arena) -> Result<Box<dyn Pool<'a> + 'a>, MpsError> {
        let format = ObjectFormat::managed_with::<TreeObject>(arena)?;
        match *self {
            PoolType::MarkSweep => Ok(Box::new(AutoMarkSweep::builder(arena).build(format)?))
        }
    }
}
fn main() {
    let n = std::env::args().nth(1)
        .and_then(|n| n.parse().ok())
        .unwrap_or(10);
    let pool_type = PoolType::MarkSweep; // TODO: Configure more than one pool type
    let min_depth = 4;
    let max_depth = if min_depth + 2 > n { min_depth + 2 } else { n };

    let plain = slog_term::PlainSyncDecorator::new(std::io::stdout());
    let logger = Logger::root(
        slog_term::FullFormat::new(plain).build().fuse(),
        o!("bench" => file!())
    );
    let arena = {
        let mut builder = VirtualMemoryArenaClass::get().builder();
        builder.arena_size = Some(32 * 1024 * 1024); // Reserve 32MB
        builder.build().expect("Failed to build MPS arena")
    };
    let thread = arena.register_thread().unwrap();
    let root = unsafe { thread.register_roots(&pool_type as *const PoolType as *mut c_void).unwrap() };
    let pool = pool_type.create(&arena).unwrap();
    let allocation_point = pool.create_allocation_point().unwrap();
    let gc = RawMpsCollector {
        arena: &arena,
        allocation_point
    };
    {
        let depth = max_depth + 1;
        let tree = unsafe { bottom_up_tree(&gc, depth).unwrap() };
        println!("stretch tree of depth {}\t check: {}", depth, item_check(&tree));
    }

    let long_lived_tree = unsafe { bottom_up_tree(&gc, max_depth).unwrap() };

    (min_depth / 2..max_depth / 2 + 1).into_iter().for_each(|half_depth| {
        let depth = half_depth * 2;
        let iterations = 1 << ((max_depth - depth + min_depth) as u32);
        let message = inner(&gc, depth, iterations).unwrap();
        println!("{}", message);
    });

    println!("long lived tree of depth {}\t check: {}", max_depth, item_check(&long_lived_tree));
    drop(gc.allocation_point);
    drop(pool);
    drop(root);
    drop(thread);
    drop(arena);
}