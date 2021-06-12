use std::hash::{Hash, Hasher};
use std::collections::HashMap;
use std::io::{Write, Read};
use crate::Entry;
use std::fmt::Debug;
use std::alloc::Layout;
use std::sync::atomic::{AtomicUsize, Ordering};
use mps::format::{RawFormatMethods, ScanState};
use std::os::raw::c_void;

use mps::from_mps_res;
use mps_sys::mps_res_t;

const ALIGNMENT: usize = std::mem::align_of::<usize>();
/// Align the specified size upwards to the next multiple of the word size
#[inline]
pub const fn align_word(size: usize) -> usize {
    (size + ALIGNMENT - 1) & !(ALIGNMENT - 1)
}
/// Align size upwards to the next multiple of the word size,
/// and additionally ensure that it's big enough to store a forwarding object.
#[inline]
pub const fn align_obj(size: usize) -> usize {
    std::cmp::max(
        align_word(size),
        align_word(ObjectVal::compute_size(
            std::mem::size_of::<ForwardingObject>()
        ))
    )
}

// Special objects
macro_rules! special_objs {
    ($($key:ident => $name:expr),*) => {
        $(pub static $key: ObjectRef = ObjectRef(&mut ObjectVal::Special { name: StringRef::from_str($name) });)*
    };
}
special_objs! {
    EMPTY => "()",
    EOF => "#[eof]",
    ERROR => "#[error]",
    TRUE => "#t",
    FALSE => "#f",
    UNDEFINED => "#[undefined]",
    TAIL => "#[tail]",
    DELETED => "#[deleted]"
}
macro_rules! delegating_impl {
    ($target:ty, |$var:ident| $convert:expr) => {
        impl Hash for $target {
            fn hash<H: Hasher>(&self, state: &mut H) {
                let $var = self;
                $convert.hash(state)
            }
        }
        impl PartialEq for $target {
            fn eq(&self, other: &Self) -> bool {
                let first = {
                    let $var = self;
                    $convert
                };
                let second = {
                    let $var = other;
                    $convert
                };
                first == second
            }
        }
        impl Eq for $target {}
        impl std::fmt::Debug for $target {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                let $var = self;
                std::fmt::Debug::fmt($convert, f)
            }
        }
    };
}

#[derive(Copy, Clone, Debug)]
pub struct SchemeType(std::mem::Discriminant<ObjectVal>);

/// The total number of allocated bytes
///
/// This must be atomic due to rust's safety ;)
static TOTAL_ALLOCATED: AtomicUsize = AtomicUsize::new(0);

// Before integration with MPS, we just leak
#[repr(C)]
pub struct ObjectRef(*mut ObjectVal);
impl ObjectRef {
    unsafe fn uninit(size: usize) -> ObjectRef {
        let v = Vec::<u8>::with_capacity(size);
        let p = v.as_ptr() as *mut ObjectVal;
        std::mem::forget(v);
        TOTAL_ALLOCATED.fetch_add(size, Ordering::AcqRel);
        ObjectRef(p)
    }
    pub fn pair(car: ObjectRef, cdr: ObjectRef) -> ObjectRef {
        let v = ObjectVal::Pair { car, cdr };
        unsafe {
            let obj = ObjectRef::uninit(v.size());
            obj.0.write(v);
            obj
        }
    }
}
delegating_impl!(ObjectRef, |r| &*r as &ObjectVal);
#[derive(Debug, Hash, Eq, PartialEq)]
#[repr(C, u8)] // See enum repr
pub enum ObjectVal {
    Pair {
        car: ObjectRef,
        cdr: ObjectRef
    },
    Symbol {
        name: ObjectRef
    },
    Integer(i64),
    Special {
        name: StringRef,
    },
    Operator(Operator),
    String(InlineStr),
    Port(Port),
    Character(char),
    Vector(InlineArray),
    Table(Table),
    Forward(ForwardingObject),
    Forward2 {
        fwd: ObjectRef
    },
    Pad1,
    Pad {
        size: usize
    }
}
impl ObjectVal {
    /// The size of this object
    ///
    /// Since a type's size can't change at runtime,
    /// this is different than `mem::size_of::<Self>`
    /// (which must conservatively give the size of the largest variant)
    fn size(&self) -> usize {
        use std::mem::{align_of, size_of};
        let (field_align, field_size) = match *self {
            ObjectVal::Pair { car: ObjectRef(_), cdr:  ObjectRef(_) } => {
                (
                    align_of::<(ObjectRef, ObjectRef)>(),
                    size_of::<(ObjectRef, ObjectRef)>()
                )
            },
            ObjectVal::Symbol { name: ObjectRef(_) } => {
                (align_of::<ObjectRef>(), size_of::<ObjectRef>())
            },
            ObjectVal::Integer(_) => {
                (align_of::<i64>(), size_of::<i64>())
            },
            ObjectVal::Special { name: ref name @ StringRef { .. } } => {
                (align_of::<StringRef>(), name.used_mem())
            },
            ObjectVal::Operator(Operator { .. }) => {
                (align_of::<Operator>(), size_of::<Operator>())
            },
            ObjectVal::String(ref s @ InlineStr { .. }) => {
                (align_of::<InlineStr>(), s.used_mem())
            },
            ObjectVal::Port(Port { .. }) => {
                (align_of::<Port>(), size_of::<Port>())
            },
            ObjectVal::Character(c) => {
                let _c: char = c;
                (align_of::<char>(), size_of::<char>())
            },
            ObjectVal::Vector(ref v @ InlineArray { .. }) => {
                (align_of::<InlineArray>(), v.used_mem())
            },
            ObjectVal::Table(ref _t @ Table { .. }) => {
                (
                    align_of::<Table>(),
                    size_of::<Table>()
                )
            },
            ObjectVal::Forward(ForwardingObject { fwd: _, size }) => {
                (
                    ALIGNMENT,
                    size
                )
            },
            ObjectVal::Forward2 { fwd: ObjectRef(_) } => {
                (
                    align_of::<ObjectRef>(),
                    size_of::<ObjectRef>()
                )
            },
            ObjectVal::Pad1 => (ALIGNMENT, 0),
            ObjectVal::Pad { size } => (ALIGNMENT, size),
        };
        debug_assert!(field_align <= ALIGNMENT);
        align_obj(ObjectVal::compute_size(field_size))
    }
    #[inline]
    const fn compute_size(field_size: usize) -> usize {
        let result = Layout::new::<u8>(); // discriminant
        result.size() + result.padding_needed_for(ALIGNMENT) + field_size
    }
}
unsafe impl RawFormatMethods for ObjectVal {
    type Obj = Self;
    const ALIGNMENT: usize = ALIGNMENT;

    unsafe extern fn class_ptr(obj: *mut Self::Obj) -> *mut c_void {
        todo!()
    }

    unsafe extern fn forward(old: *mut Self::Obj, new: *mut Self::Obj) {
        todo!()
    }

    unsafe extern fn is_forwarded(old: *mut Self::Obj) -> *mut Self::Obj {
        todo!()
    }

    unsafe extern fn pad(addr: *mut Self::Obj, size: usize) {
        todo!()
    }

    unsafe extern fn scan(state: ScanState, mut base: *mut ObjectVal, limit: *mut Self::Obj) -> mps_res_t {
        state.fix_with(|state| {
            while base < limit {
                let mut size = align_obj((*base).size());
                match *base {
                    ObjectVal::Pair { ref mut car, ref mut cdr } => {
                        state.fix(&mut car.0)?;
                        state.fix(&mut cdr.0)?;
                    },
                    ObjectVal::Integer(_) => {},
                    ObjectVal::Symbol { name } => {
                        state.fix(&mut name.raw_bytes)?;
                    }
                    ObjectVal::Special { .. } => {}
                    ObjectVal::Operator(ref mut op) => {
                        state.fix(&mut op.arguments.0)?;
                        state.fix(&mut op.body.0)?;
                        state.fix(&mut op.env.0)?;
                        state.fix(&mut op.op_env.0)?;
                    }
                    ObjectVal::String(_) => {}
                    ObjectVal::Port(ref mut p) => {
                        state.fix(&mut p.name.0)?;
                    }
                    ObjectVal::Character(_) => {}
                    ObjectVal::Vector(_) => {}
                    ObjectVal::Table(_) => {},

                }
                base = base.add(size);
            }
            Ok(())
        })
    }

    unsafe extern fn skip(addr: *mut Self::Obj) -> *mut Self::Obj {
        todo!()
    }
}
#[derive(Debug, Eq, PartialEq)]
pub struct Table {
    // NOTE: Must have indirection for FFI-safety
    map: Box<HashMap<ObjectRef, ObjectRef>>
}
impl Hash for Table {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut entries = self.map.iter()
            .collect::<Vec<_>>();
        entries.sort_by_key(|(key, _)| {
            // Sort by address. It's stupid (but deterministic)
            key.0 as *const ObjectVal as usize
        });
        for entry in entries {
            entry.hash(state);
        }
    }
}
#[derive(Debug, Eq, PartialEq, Hash)]
#[repr(C)]
pub struct Port {
    name: ObjectRef,
    stream: PortStream
}
#[repr(C)]
pub struct StringRef {
    length: usize,
    bytes: *const u8
}
impl StringRef {
    pub const fn from_str(s: &'static str) -> StringRef {
        StringRef {
            length: s.len(),
            bytes: s.as_ptr()
        }
    }
    #[inline]
    pub fn used_mem(&self) -> usize {
        // NOTE: We are a *reference*
        std::mem::size_of::<Self>()
    }
    #[inline]
    pub fn as_str(&self) -> &str {
        unsafe {
            std::str::from_utf8_unchecked(std::slice::from_raw_parts(
                self.bytes, self.length
            ))
        }
    }
}
delegating_impl!(StringRef, |s| s.as_str());
#[repr(C)]
struct ForwardingObject {
    pub fwd: ObjectRef,
    pub size: usize
}
delegating_impl!(ForwardingObject, |obj| obj.fwd);
#[derive(Debug, Eq, PartialEq, Hash)]
#[repr(C)]
pub struct Operator {
    pub name: StringRef,
    pub entry: Entry,
    pub arguments: ObjectRef,
    pub body: ObjectRef,
    pub env: ObjectRef,
    pub op_env: ObjectRef
}
#[repr(C)]
pub struct InlineStr {
    length: usize,
    raw_bytes: [u8; 0]
}
impl InlineStr {
    /// Size of used memory
    #[inline]
    pub fn used_mem(&self) -> usize {
        use std::mem::size_of;
        size_of::<Self>() + (self.length * size_of::<u8>())
    }
    #[inline]
    pub fn as_str(&self) -> &str {
        unsafe {
            std::str::from_utf8_unchecked(std::slice::from_raw_parts(
                self.raw_bytes.as_ptr(), self.length
            ))
        }
    }
}
delegating_impl!(InlineStr, |s| s.as_str());
#[repr(C)]
pub struct InlineArray {
    length: usize,
    raw_elements: [ObjectRef; 0]
}
impl InlineArray {
    /// Size of used memory
    #[inline]
    pub fn used_mem(&self) -> usize {
        use std::mem::size_of;
        size_of::<Self>() + (self.length * size_of::<ObjectRef>())
    }
    #[inline]
    fn as_slice(&self) -> &[ObjectRef] {
        unsafe {
            std::slice::from_raw_parts(self.raw_elements.as_ptr(), self.length)
        }
    }
}
delegating_impl!(InlineArray, |a| a.as_slice());

#[repr(C)]
pub enum PortStream {
    // NOTE: Need double-indirection for FFI safety
    Input(Box<Box<dyn Read + Sync>>),
    Output(Box<Box<dyn Write + Sync>>)
}
impl PortStream {
    fn addr(&self) -> usize {
        match self {
            PortStream::Input(b) => {
                b.as_ref() as *const Box<_> as usize
            },
            PortStream::Output(b) => {
                b.as_ref() as *const Box<_> as usize
            },
        }
    }
}
delegating_impl!(PortStream, |s| &s.addr());
