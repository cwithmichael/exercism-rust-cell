mod reactor_cell;
pub use crate::reactor_cell::cell::{self, Cell};
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
    cells: HashMap<CellId, Rc<RefCell<cell::ReactorCell<'a, T>>>>,
    compute_cells: HashMap<CellId, Rc<RefCell<cell::ReactorCell<'a, T>>>>,
    rng: ThreadRng,
}

// You are guaranteed that Reactor will only be tested against types that are Copy + PartialEq.
impl<'a, T: Copy + PartialEq> Reactor<'a, T> {
    pub fn new() -> Self {
        Reactor {
            cells: HashMap::new(),
            compute_cells: HashMap::new(),
            rng: rand::thread_rng(),
        }
    }

    /* pub fn notify_compute_cells(
        compute_cells: &HashMap<CellId, Rc<RefCell<cell::ReactorCell<T>>>>,
    ) {
        for cell in compute_cells {
            let dep_cell = &mut *cell.1.borrow_mut();
            match &mut *dep_cell {
                cell::ReactorCell::ComputeCell(ref mut c) => c.update_value(),
                _ => (),
            }
        }
    }*/

    // Creates an input cell with the specified initial value, returning its ID.
    pub fn create_input(&mut self, initial: T) -> InputCellId {
        let gen_id: i32 = self.rng.gen();
        let ici = InputCellId(gen_id);
        self.cells.insert(
            CellId::Input(ici),
            Rc::new(RefCell::new(cell::ReactorCell::InputCell(
                cell::InputCell { value: initial },
            ))),
        );
        ici
    }

    // Creates a compute cell with the specified dependencies and compute function.
    // The compute function is expected to take in its arguments in the same order as specified in
    // `dependencies`.
    // You do not need to reject compute functions that expect more arguments than there are
    // dependencies (how would you check for this, anyway?).
    //
    // If any dependency doesn't exist, returns an Err with that nonexistent dependency.
    // (If multiple dependencies do not exist, exactly which one is returned is not defined and
    // will not be tested)
    //
    // Notice that there is no way to *remove* a cell.
    // This means that you may assume, without checking, that if the dependencies exist at creation
    // time they will continue to exist as long as the Reactor exists.
    pub fn create_compute<F: Fn(&[T]) -> T>(
        &mut self,
        dependencies: &[CellId],
        compute_func: F,
    ) -> Result<ComputeCellId, CellId>
    where
        F: 'static,
    {
        let gen_id: i32 = self.rng.gen();
        let cci = ComputeCellId(gen_id);

        let mut deps = vec![];
        let mut cell_deps = vec![];
        for dep in dependencies {
            match self.cells.get(dep) {
                Some(input_cell) => {
                    let c = input_cell.borrow();
                    deps.push(c.value());
                    cell_deps.push(Rc::clone(input_cell));
                }
                None => match self.compute_cells.get(dep) {
                    Some(compute_cell) => {
                        let c = compute_cell.borrow();
                        deps.push(c.value());
                        cell_deps.push(Rc::clone(compute_cell));
                    }
                    None => return Err(*dep),
                },
            }
        }

        self.compute_cells.insert(
            CellId::Compute(cci),
            Rc::new(RefCell::new(cell::ReactorCell::ComputeCell(
                cell::ComputeCell {
                    value: compute_func(&deps),
                    compute_func: Box::new(compute_func),
                    callbacks: None,
                    cell_deps: cell_deps,
                },
            ))),
        );
        Ok(cci)
    }

    // Retrieves the current value of the cell, or None if the cell does not exist.
    //
    // You may wonder whether it is possible to implement `get(&self, id: CellId) -> Option<&Cell>`
    // and have a `value(&self)` method on `Cell`.
    //
    // It turns out this introduces a significant amount of extra complexity to this exercise.
    // We chose not to cover this here, since this exercise is probably enough work as-is.
    pub fn value(&self, id: CellId) -> Option<T> {
        match id {
            CellId::Compute(_) => match self.compute_cells.get(&id) {
                Some(cell) => {
                    let c = cell.borrow();
                    Some(c.value())
                }
                None => None,
            },
            CellId::Input(_) => match self.cells.get(&id) {
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
    //
    // Similarly, you may wonder about `get_mut(&mut self, id: CellId) -> Option<&mut Cell>`, with
    // a `set_value(&mut self, new_value: T)` method on `Cell`.
    //
    // As before, that turned out to add too much extra complexity.
    pub fn set_value(&mut self, id: InputCellId, new_value: T) -> bool {
        match self.cells.get_mut(&CellId::Input(id)) {
            Some(cell) => match *cell.borrow_mut() {
                cell::ReactorCell::InputCell(ref mut ic) => {
                    ic.set_value(new_value);
                }
                _ => return false,
            },
            None => return false,
        }
        // Notify compute cells
        for cell in &self.compute_cells {
            let compute_cell = &mut *cell.1.borrow_mut();
            match &mut *compute_cell {
                cell::ReactorCell::ComputeCell(ref mut c) => {
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
                cell::ReactorCell::ComputeCell(ref mut cc) => {
                    //cc.set_value(new_value);
                    let gen_id: i32 = self.rng.gen();
                    let cbi = CallbackId(gen_id);
                    match cc.callbacks.as_mut() {
                        Some(cbs) => {
                            cbs.push(Rc::new(RefCell::new(callback)));
                        }
                        None => {
                            cc.callbacks = Some(vec![Rc::new(RefCell::new(callback))]);
                        }
                    }
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
        unimplemented!(
            "Remove the callback identified by the CallbackId {:?} from the cell {:?}",
            callback,
            cell,
        )
    }
}
