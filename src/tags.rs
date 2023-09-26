use std::fmt::Debug;
use std::ops::Deref;
use std::ops::DerefMut;

#[derive(Debug, PartialEq, Eq)]
pub struct Full;

#[derive(Debug, PartialEq, Eq)]
pub struct Empty;

pub struct Tag<T: Clone + Default + PartialEq + Eq, Data, State> {
    phantom_tag: std::marker::PhantomData<T>,
    data: Option<Data>,
    phantom_state: std::marker::PhantomData<State>,
}

impl<T: Clone + Default + PartialEq + Eq, Data, State> PartialEq for Tag<T, Data, State> {
    fn eq(&self, other: &Self) -> bool {
        self.phantom_tag == other.phantom_tag
    }
}

impl<T: Clone + Default + Debug + PartialEq + Eq, Data, State> Eq for Tag<T, Data, State> {}

impl<T: Clone + Default + PartialEq + Eq, Data: Debug, State> Debug for Tag<T, Data, State> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tag")
            .field("tag", &self.phantom_tag)
            .field("data", &self.data)
            .field("state", &self.phantom_state)
            .finish()
    }
}

impl<T: Clone + Default + PartialEq + Eq, D> Default for Tag<T, D, Empty> {
    fn default() -> Self {
        Self {
            phantom_tag: std::marker::PhantomData,
            data: None,
            phantom_state: std::marker::PhantomData,
        }
    }
}

impl<T: Clone + Default + PartialEq + Eq, D> Tag<T, D, Full> {
    pub fn new(data: D) -> Self {
        Self {
            phantom_tag: std::marker::PhantomData,
            data: Some(data),
            phantom_state: std::marker::PhantomData,
        }
    }
}

impl<T: Clone + Default + PartialEq + Eq, D> Deref for Tag<T, D, Full> {
    type Target = D;

    fn deref(&self) -> &Self::Target {
        self.data.as_ref().unwrap()
    }
}

impl<T: Clone + Default + PartialEq + Eq, D> DerefMut for Tag<T, D, Full> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data.as_mut().unwrap()
    }
}
