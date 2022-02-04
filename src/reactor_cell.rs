pub mod cell {
  use std::cell::RefCell;
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

  pub struct ComputeCell<T: Copy> {
    pub value: T,
    pub compute_func: Box<dyn Fn(&[T]) -> T>,
    pub cell_deps: Vec<Rc<RefCell<ReactorCell<T>>>>,
  }

  impl<T: Copy> ComputeCell<T> {
    pub fn update_value(&mut self) {
      let mut args = vec![];
      for dep in &self.cell_deps {
        let dep_cell = &*dep.borrow();
        args.push(dep_cell.value());
      }
      self.value = (self.compute_func)(&args)
    }
  }

  impl<T: Copy> Cell<T> for ComputeCell<T> {
    fn value(&self) -> T {
      self.value
    }
  }

  pub enum ReactorCell<T: Copy> {
    InputCell(InputCell<T>),
    ComputeCell(ComputeCell<T>),
  }

  impl<T: Copy> Cell<T> for ReactorCell<T> {
    fn value(&self) -> T {
      match self {
        ReactorCell::InputCell(ic) => ic.value(),
        ReactorCell::ComputeCell(cc) => cc.value(),
      }
    }
  }
}
