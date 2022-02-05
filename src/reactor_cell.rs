use crate::CallbackId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub trait Cell<T> {
  fn value(&self) -> T;
}

#[derive(Copy, Clone)]
pub struct InputCell<T: Copy> {
  pub value: T,
}

impl<T: Copy> InputCell<T> {
  pub fn set_value(&mut self, value: T) {
    self.value = value;
  }
}

impl<T: Copy> Cell<T> for InputCell<T> {
  fn value(&self) -> T {
    self.value
  }
}

pub struct ComputeCell<'a, T: Copy + PartialEq> {
  pub value: T,
  pub compute_func: Box<dyn Fn(&[T]) -> T>,
  pub callbacks: HashMap<CallbackId, Rc<RefCell<dyn FnMut(T) + 'a>>>,
  pub cell_deps: Vec<Rc<RefCell<ReactorCell<'a, T>>>>,
}

impl<'a, T: Copy + PartialEq> ComputeCell<'a, T> {
  pub fn add_callback(&mut self, cb_id: CallbackId, cb: Rc<RefCell<dyn FnMut(T)>>) {
    self.callbacks.insert(cb_id, cb);
  }
  pub fn update_value(&mut self) {
    let mut args = vec![];
    for dep in &self.cell_deps {
      let dep_cell = &mut *dep.borrow_mut();
      // Could be an Input or Compute Cell
      match dep_cell {
        ReactorCell::ComputeCell(ref mut c) => {
          c.update_value();
          args.push(c.value());
        }
        ReactorCell::InputCell(i) => {
          args.push(i.value());
        }
      }
    }
    // Store the new value to check it against the previous one
    let new_value = (self.compute_func)(&args);
    if self.value != new_value {
      for (_, cb) in &self.callbacks {
        let mut callback = cb.borrow_mut();
        callback(new_value);
      }
    }
    self.value = new_value;
  }
}

impl<'a, T: Copy + PartialEq> Cell<T> for ComputeCell<'a, T> {
  fn value(&self) -> T {
    self.value
  }
}

pub enum ReactorCell<'a, T: Copy + PartialEq> {
  InputCell(InputCell<T>),
  ComputeCell(ComputeCell<'a, T>),
}

impl<'a, T: Copy + PartialEq> Cell<T> for ReactorCell<'a, T> {
  fn value(&self) -> T {
    match self {
      ReactorCell::InputCell(ic) => ic.value(),
      ReactorCell::ComputeCell(cc) => cc.value(),
    }
  }
}
