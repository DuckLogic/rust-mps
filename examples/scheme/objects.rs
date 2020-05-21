use std::hash::{Hash, Hasher};
use std::collections::HashMap;
use std::io::{Write, Read};
use crate::Entry;
use std::fmt::Debug;
use std::alloc::Layout;
use std::sync::atomic::AtomicUsize;

// Special objects
macro_rules! special_objs {
    ($($key:ident => $name:expr),*) => {
        $(pub static $key: ObjectRef = ObjectRef(&ObjectVal::Special { name: StringRef::from_str($name) });)*
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
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
#[repr(C)]
pub struct ObjectRef(&'static ObjectVal);
impl ObjectRef {
    unsafe fn uninit(size: usize) -> ObjectRef {
        let v = Vec::with_capacity(size);
        let p = v.as_ptr() as *mut ObjectVal;
        std::mem::forget(v);
        TOTAL_ALLOCATED.fetch_add(size);
        ObjectRef(unsafe { &*p })
    }
    pub fn pair(first: ObjectRef, second: ObjectRef) -> ObjectRef {
        let v = ObjectVal::Pair(first, second);
        unsafe {
            let obj = ObjectRef::uninit(v.size());
            obj.0.write(v);
            obj
        }
    }
}
#[derive(Debug, Hash, Eq, PartialEq)]
#[repr(C, u8)] // See enum repr
pub enum ObjectVal {
    Pair(ObjectRef, ObjectRef),
    Symbol {
        name: InlineStr
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
    Table(Table)
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
            ObjectVal::Pair(ObjectRef(_), ObjectRef(_)) => {
                (
                    align_of::<(ObjectRef, ObjectRef)>(),
                    size_of::<(ObjectRef, ObjectRef)>()
                )
            },
            ObjectVal::Symbol { ref name } => {
                (align_of::<InlineStr>(), name.used_mem())
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
            }
        };
        let result = Layout::new::<u8>(); // discriminant
        result.size() + result.padding_needed_for(field_align) + field_size

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
#[derive(Debug, Eq, PartialEq, Hash)]
#[repr(C)]
pub struct Operator {
    pub name: StringRef,
    pub entry: Entry,
    pub arguments: ObjectRef,
    pub body: ObjectRef,
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
