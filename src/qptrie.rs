use crate::prelude::*;

enum CookedTwig<'t, T>
where
    T: Sized,
{
    Branch { offset: usize, twigs: BmpVec<RawTwig<'t, T>> },
    Leaf { key: HeapName, val: &'t mut T },
}

struct RawTwig<'t, T> {
    int: u64,
    ptr: *mut (),
    _marker: PhantomData<&'t mut T>,
}

impl<'t, T> From<RawTwig<'t, T>> for CookedTwig<'t, T> {
    fn from(raw: RawTwig<'t, T>) -> CookedTwig<'t, T> {
        unsafe {
            if raw.int & BRANCH_TAG == 0 {
                let key = HeapName::from_raw_parts(raw.int as *const u8);
                let val = &mut *(raw.ptr as *mut T);
                CookedTwig::Leaf { key, val }
            } else {
                let offset = (raw.int >> SHIFT_OFFSET) as usize;
                let bmp = raw.int & MASK_BMP;
                let ptr = raw.ptr as *mut RawTwig<'t, T>;
                let twigs = BmpVec::from_raw_parts(bmp, ptr);
                CookedTwig::Branch { offset, twigs }
            }
        }
    }
}

impl<'t, T> From<CookedTwig<'t, T>> for RawTwig<'t, T> {
    fn from(twig: CookedTwig<'t, T>) -> RawTwig<'t, T> {
        unsafe {
            match twig {
                CookedTwig::Branch { offset, twigs } => {
                    let (bmp, ptr) = twigs.into_raw_parts();
                    let ptr = ptr as *mut ();
                    let off = offset as u64;
                    let int = BRANCH_TAG | bmp | off << SHIFT_OFFSET;
                    RawTwig { int, ptr, _marker: PhantomData }
                }
                CookedTwig::Leaf { key, val } => {
                    let int = key.into_ptr() as u64;
                    let ptr = val as *mut T as *mut ();
                    RawTwig { int, ptr, _marker: PhantomData }
                }
            }
        }
    }
}
