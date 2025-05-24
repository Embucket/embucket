use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

pub trait Generator<T> {
    // create entity, item index is just for reference
    fn generate(&self, index: usize) -> T;
}

#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct WithCount<T, G>
where
    G: Generator<T>,
{
    count: usize,
    template: G,
    #[serde(skip)]
    _marker: PhantomData<T>,
}

impl<T, G> WithCount<T, G>
where
    G: Generator<T>,
{
    #[must_use]
    pub const fn new(count: usize, template: G) -> Self {
        Self {
            count,
            template,
            _marker: PhantomData,
        }
    }

    // create items for template, item index is just for reference
    pub fn vec_with_count(&self, _index: usize) -> Vec<T> {
        // call generate n times
        (0..self.count).map(|i| self.template.generate(i)).collect()
    }
}
