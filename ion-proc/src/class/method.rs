/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use proc_macro2::Ident;
use syn::{ItemFn, Result, Signature};

use crate::function::{check_abi, error_handler, set_signature};
use crate::function::parameters::Parameters;
use crate::function::wrapper::impl_wrapper_fn;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum MethodKind {
	Constructor,
	Getter,
	Setter,
	Internal,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum MethodReceiver {
	Dynamic,
	Static,
}

#[derive(Clone, Debug)]
pub(crate) struct Method {
	pub(crate) receiver: MethodReceiver,
	pub(crate) method: ItemFn,
	pub(crate) inner: Option<ItemFn>,
	pub(crate) nargs: usize,
	pub(crate) aliases: Vec<Ident>,
}

pub(crate) fn impl_method<F>(mut method: ItemFn, ident: &Ident, keep_inner: bool, predicate: F) -> Result<(Method, Parameters)>
where
	F: FnOnce(&Signature) -> Result<()>,
{
	let krate = quote!(::ion);
	let (wrapper, mut inner, parameters) = impl_wrapper_fn(method.clone(), Some(ident), keep_inner, false)?;

	predicate(&method.sig).and_then(|_| {
		check_abi(&mut method)?;
		set_signature(&mut method)?;
		method.attrs.clear();
		method.attrs.push(parse_quote!(#[allow(non_snake_case)]));

		if !keep_inner {
			inner.attrs.push(parse_quote!(#[allow(non_snake_case)]));
		}

		let error_handler = error_handler();

		let body = parse_quote!({
			let args = #krate::Arguments::new(argc, vp);
			#wrapper
			let result = ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| wrapper(cx, &args)));
			#error_handler
		});
		method.block = Box::new(body);

		let method = Method {
			receiver: if parameters.this.is_some() {
				MethodReceiver::Dynamic
			} else {
				MethodReceiver::Static
			},
			method,
			inner: if !keep_inner { Some(inner) } else { None },
			nargs: parameters.nargs.0,
			aliases: vec![],
		};
		Ok((method, parameters))
	})
}
