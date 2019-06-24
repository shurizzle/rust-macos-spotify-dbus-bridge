use std::fmt;
use std::sync::Arc;
use std::sync::RwLock;

pub struct ATracked<T>
where
    T: PartialEq,
{
    inner: RwLock<(Arc<T>, bool)>,
}

impl<T: PartialEq> ATracked<T> {
    pub fn new(value: T) -> ATracked<T> {
        ATracked {
            inner: RwLock::new((Arc::new(value), false)),
        }
    }

    pub fn get(&self) -> Arc<T> {
        (*self.inner.read().unwrap()).0.clone()
    }

    pub fn set(&self, value: T) {
        if !PartialEq::eq(self.get().as_ref(), &value) {
            let value = Arc::new(value);
            {
                let mut obj = self.inner.write().unwrap();
                *obj = (value, true);
            }
        }
    }

    pub fn reset(&self) {
        let mut obj = self.inner.write().unwrap();
        (*obj).1 = false;
    }

    pub fn has_changed(&self) -> bool {
        (*self.inner.read().unwrap()).1
    }
}

impl<T: PartialEq + fmt::Debug> fmt::Debug for ATracked<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ATracked")
            .field("value", &self.get())
            .field("changed", &self.has_changed())
            .finish()
    }
}

impl<T: PartialEq> Default for ATracked<Option<T>> {
    fn default() -> Self {
        ATracked::new(None)
    }
}

unsafe impl<T: PartialEq> Send for ATracked<T> {}
unsafe impl<T: PartialEq> Sync for ATracked<T> {}
