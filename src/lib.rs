use std::ops::{Deref, DerefMut};

pub struct Guard<T, F> where
    F: FnMut(&mut T)
{
    __dropfn: F,
    __value: T,
}

pub fn guard<T, F>(v: T, dropfn: F) -> Guard<T, F> where
    F: FnMut(&mut T)
{
    Guard{__value: v, __dropfn: dropfn}
}

impl<T, F> Deref for Guard<T, F> where
    F: FnMut(&mut T)
{
    type Target = T;
    fn deref(&self) -> &T
    {
        &self.__value
    }

}

impl<T, F> DerefMut for Guard<T, F> where
    F: FnMut(&mut T)
{
    fn deref_mut(&mut self) -> &mut T
    {
        &mut self.__value
    }
}

impl<T, F> Drop for Guard<T, F> where
    F: FnMut(&mut T)
{
    fn drop(&mut self) {
        (self.__dropfn)(&mut self.__value)
    }
}

