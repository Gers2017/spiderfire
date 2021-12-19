/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::collections::hash_map::Entry;
use std::rc::Rc;

use ion::IonContext;

use crate::event_loop::macrotasks::{Macrotask, MacrotaskQueue};
use crate::event_loop::microtasks::MicrotaskQueue;

pub mod macrotasks;
pub mod microtasks;

pub struct EventLoop {
	macrotasks: Option<Rc<MacrotaskQueue>>,
	microtasks: Option<Rc<MicrotaskQueue>>,
}

impl EventLoop {
	pub fn new(macrotasks: Option<Rc<MacrotaskQueue>>, microtasks: Option<Rc<MicrotaskQueue>>) -> EventLoop {
		EventLoop { macrotasks, microtasks }
	}

	pub fn run(&self, cx: IonContext) -> Result<(), ()> {
		if self.macrotasks.is_none() && self.microtasks.is_none() {
			return Ok(());
		}

		let mut result = Ok(());

		if let Some(microtasks) = self.microtasks.clone() {
			let run = microtasks.run_jobs(cx);
			if run.is_err() {
				result = run;
			}
		}

		if let Some(macrotasks) = self.macrotasks.clone() {
			while !macrotasks.map.borrow().is_empty() {
				if let Some(next) = macrotasks.next() {
					let macrotask_map = macrotasks.map.borrow();
					let macrotask = macrotask_map.get(&next).cloned();
					drop(macrotask_map);
					if let Some(macrotask) = macrotask {
						let macrotask = macrotask.clone();
						let run = macrotask.run(cx);
						if run.is_err() {
							result = Err(());
						}

						if let Entry::Occupied(mut entry) = macrotasks.map.borrow_mut().entry(next) {
							if let Macrotask::Timer(ref mut timer) = entry.get_mut() {
								if !timer.reset() {
									entry.remove();
								}
							} else {
								entry.remove();
							}
						}
					}
				}
				macrotasks.find_next();

				if let Some(microtasks) = self.microtasks.clone() {
					let run = microtasks.run_jobs(cx);
					if run.is_err() {
						result = run;
					}
				}
			}
		}

		result
	}
}
