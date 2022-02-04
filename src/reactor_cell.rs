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

  pub struct ComputeCell<'a, T: Copy> {
    pub value: T,
    pub compute_func: Box<dyn Fn(&[T]) -> T>,
    pub callbacks: Option<Vec<Rc<dyn FnMut(T) + 'a>>>,
    pub cell_deps: Vec<Rc<RefCell<ReactorCell<'a, T>>>>,
  }

  impl<'a, T: Copy> ComputeCell<'a, T> {
    pub fn add_callback<F: FnMut(T)>(&mut self, cb: Rc<dyn FnMut(T)>) {
      self.callbacks.as_mut().unwrap().push(cb);
    }
    pub fn update_value(&mut self) {
      let mut args = vec![];
      for dep in &self.cell_deps {
        let dep_cell = &mut *dep.borrow_mut();
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
      self.value = (self.compute_func)(&args);
      match &self.callbacks {
        Some(cbs) => {
          for cb in cbs {
            //cb(3)
          }
        }
        None => (),
      }
    }
  }

  impl<'a, T: Copy> Cell<T> for ComputeCell<'a, T> {
    fn value(&self) -> T {
      self.value
    }
  }

  pub enum ReactorCell<'a, T: Copy> {
    InputCell(InputCell<T>),
    ComputeCell(ComputeCell<'a, T>),
  }

  impl<'a, T: Copy> Cell<T> for ReactorCell<'a, T> {
    fn value(&self) -> T {
      match self {
        ReactorCell::InputCell(ic) => ic.value(),
        ReactorCell::ComputeCell(cc) => cc.value(),
      }
    }
  }
}
