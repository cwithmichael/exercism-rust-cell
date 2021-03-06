mod reactor_cell;
pub use crate::reactor_cell::*;
use rand::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// `InputCellId` is a unique identifier for an input cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InputCellId(i32);
/// `ComputeCellId` is a unique identifier for a compute cell.
/// Values of type `InputCellId` and `ComputeCellId` should not be mutually assignable,
/// demonstrated by the following tests:
///
/// ```compile_fail
/// let mut r = react::Reactor::new();
/// let input: react::ComputeCellId = r.create_input(111);
/// ```
///
/// ```compile_fail
/// let mut r = react::Reactor::new();
/// let input = r.create_input(111);
/// let compute: react::InputCellId = r.create_compute(&[react::CellId::Input(input)], |_| 222).unwrap();
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ComputeCellId(i32);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CallbackId(i32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CellId {
    Input(InputCellId),
    Compute(ComputeCellId),
}

#[derive(Debug, PartialEq)]
pub enum RemoveCallbackError {
    NonexistentCell,
    NonexistentCallback,
}

pub struct Reactor<'a, T: Copy + PartialEq> {
    input_cells: HashMap<CellId, Rc<RefCell<ReactorCell<'a, T>>>>,
    compute_cells: HashMap<CellId, Rc<RefCell<ReactorCell<'a, T>>>>,
    rng: ThreadRng,
}

// You are guaranteed that Reactor will only be tested against types that are Copy + PartialEq.
impl<'a, T: Copy + PartialEq> Reactor<'a, T> {
    pub fn new() -> Self {
        Reactor {
            input_cells: HashMap::new(),
            compute_cells: HashMap::new(),
            rng: rand::thread_rng(),
        }
    }
    // Creates an input cell with the specified initial value, returning its ID.
    pub fn create_input(&mut self, initial: T) -> InputCellId {
        let ici = InputCellId(self.rng.gen());
        self.input_cells.insert(
            CellId::Input(ici),
            Rc::new(RefCell::new(ReactorCell::InputCell(InputCell {
                value: initial,
            }))),
        );
        ici
    }

    // Creates a compute cell with the specified dependencies and compute function.
    // The compute function is expected to take in its arguments in the same order as specified in
    // `dependencies`.
    //
    // If any dependency doesn't exist, returns an Err with that nonexistent dependency.
    // (If multiple dependencies do not exist, exactly which one is returned is not defined and
    // will not be tested)
    //
    // Notice that there is no way to *remove* a cell.
    // This means that you may assume, without checking, that if the dependencies exist at creation
    // time they will continue to exist as long as the Reactor exists.
    pub fn create_compute<F: 'static + Fn(&[T]) -> T>(
        &mut self,
        dependencies: &[CellId],
        compute_func: F,
    ) -> Result<ComputeCellId, CellId> {
        let mut func_args = vec![];
        let mut cell_deps = vec![];
        for dep in dependencies {
            match self.input_cells.get(dep) {
                Some(input_cell) => {
                    let c = input_cell.borrow();
                    func_args.push(c.value());
                    cell_deps.push(Rc::clone(input_cell));
                }
                None => match self.compute_cells.get(dep) {
                    Some(compute_cell) => {
                        let c = compute_cell.borrow();
                        func_args.push(c.value());
                        cell_deps.push(Rc::clone(compute_cell));
                    }
                    None => return Err(*dep),
                },
            }
        }
        let cci = ComputeCellId(self.rng.gen());
        self.compute_cells.insert(
            CellId::Compute(cci),
            Rc::new(RefCell::new(ReactorCell::ComputeCell(ComputeCell {
                value: compute_func(&func_args),
                compute_func: Box::new(compute_func),
                callbacks: HashMap::new(),
                cell_deps: cell_deps,
            }))),
        );
        Ok(cci)
    }

    // Retrieves the current value of the cell, or None if the cell does not exist.
    pub fn value(&self, id: CellId) -> Option<T> {
        match id {
            CellId::Compute(_) => match self.compute_cells.get(&id) {
                Some(cell) => {
                    let c = cell.borrow();
                    Some(c.value())
                }
                None => None,
            },
            CellId::Input(_) => match self.input_cells.get(&id) {
                Some(cell) => {
                    let c = cell.borrow();
                    Some(c.value())
                }
                None => None,
            },
        }
    }

    // Sets the value of the specified input cell.
    //
    // Returns false if the cell does not exist.
    pub fn set_value(&mut self, id: InputCellId, new_value: T) -> bool {
        match self.input_cells.get_mut(&CellId::Input(id)) {
            Some(cell) => match *cell.borrow_mut() {
                ReactorCell::InputCell(ref mut ic) => {
                    ic.set_value(new_value);
                }
                _ => return false,
            },
            None => return false,
        }
        // Notify compute cells
        for (_, cell) in &self.compute_cells {
            let compute_cell = &mut *cell.borrow_mut();
            match &mut *compute_cell {
                ReactorCell::ComputeCell(ref mut c) => {
                    c.update_value();
                }
                _ => (),
            }
        }
        true
    }

    // Adds a callback to the specified compute cell.
    //
    // Returns the ID of the just-added callback, or None if the cell doesn't exist.
    //
    // Callbacks on input cells will not be tested.
    //
    // The semantics of callbacks (as will be tested):
    // For a single set_value call, each compute cell's callbacks should each be called:
    // * Zero times if the compute cell's value did not change as a result of the set_value call.
    // * Exactly once if the compute cell's value changed as a result of the set_value call.
    //   The value passed to the callback should be the final value of the compute cell after the
    //   set_value call.
    pub fn add_callback<F: 'a + FnMut(T)>(
        &mut self,
        id: ComputeCellId,
        callback: F,
    ) -> Option<CallbackId> {
        match self.compute_cells.get_mut(&CellId::Compute(id)) {
            Some(cell) => match *cell.borrow_mut() {
                ReactorCell::ComputeCell(ref mut cc) => {
                    let cbi = CallbackId(self.rng.gen());
                    cc.callbacks.insert(cbi, Rc::new(RefCell::new(callback)));
                    Some(cbi)
                }
                _ => None,
            },
            None => None,
        }
    }

    // Removes the specified callback, using an ID returned from add_callback.
    //
    // Returns an Err if either the cell or callback does not exist.
    //
    // A removed callback should no longer be called.
    pub fn remove_callback(
        &mut self,
        cell: ComputeCellId,
        callback: CallbackId,
    ) -> Result<(), RemoveCallbackError> {
        match self.compute_cells.get(&CellId::Compute(cell)) {
            Some(compute_cell) => match *compute_cell.borrow_mut() {
                ReactorCell::ComputeCell(ref mut cc) => match cc.callbacks.remove(&callback) {
                    Some(_) => Ok(()),
                    None => Err(RemoveCallbackError::NonexistentCallback),
                },
                _ => Err(RemoveCallbackError::NonexistentCell),
            },
            None => Err(RemoveCallbackError::NonexistentCell),
        }
    }
}
