/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::collections::HashMap;

use convert_case::{Case, Casing};
use proc_macro2::Ident;
use syn::{ItemImpl, ItemStatic, ItemStruct, LitStr};

use crate::class::accessor::Accessor;
use crate::class::method::Method;
use crate::class::property::Property;

pub(crate) fn class_spec(object: &ItemStruct) -> ItemStatic {
	let name = LitStr::new(&format!("{}\0", object.ident), object.ident.span());

	parse_quote!(
		static CLASS: ::mozjs::jsapi::JSClass = ::mozjs::jsapi::JSClass {
			name: #name.as_ptr() as *const ::core::primitive::i8,
			flags: ::mozjs::jsapi::JSCLASS_HAS_PRIVATE,
			cOps: ::std::ptr::null_mut(),
			spec: ::std::ptr::null_mut(),
			ext: ::std::ptr::null_mut(),
			oOps: ::std::ptr::null_mut(),
		};
	)
}

pub(crate) fn methods_to_specs(methods: &[Method], stat: bool) -> ItemStatic {
	let krate = quote!(::ion);
	let ident = if stat { quote!(STATIC_FUNCTIONS) } else { quote!(FUNCTIONS) };
	let mut specs: Vec<_> = methods
		.iter()
		.flat_map(|method| {
			let ident = method.method.sig.ident.clone();
			let nargs = method.nargs as u16;
			method
				.aliases
				.iter()
				.map(|alias| {
					let span = alias.span();
					let mut alias = alias.to_string();
					if alias.is_case(Case::Snake) {
						alias = alias.to_case(Case::Camel);
					}
					let name = LitStr::new(&alias, span);
					quote!(#krate::function_spec!(#ident, #name, #nargs, #krate::flags::PropertyFlags::CONSTANT))
				})
				.collect::<Vec<_>>()
		})
		.collect();
	specs.push(quote!(::mozjs::jsapi::JSFunctionSpec::ZERO));

	parse_quote!(
		static #ident: &[::mozjs::jsapi::JSFunctionSpec] = &[
			#(#specs),*
		];
	)
}

pub(crate) fn properties_to_specs(properties: &[Property], accessors: &HashMap<String, Accessor>, class: &Ident, stat: bool) -> ItemStatic {
	let ident: Ident = if stat {
		parse_quote!(STATIC_PROPERTIES)
	} else {
		parse_quote!(PROPERTIES)
	};

	let mut specs: Vec<_> = properties.iter().map(|property| property.to_spec(class.clone())).collect();
	accessors
		.iter()
		.for_each(|(name, accessor)| specs.push(accessor.to_spec(Ident::new(name, class.span()))));

	specs.push(quote!(::mozjs::jsapi::JSPropertySpec::ZERO));

	parse_quote!(
		static #ident: &[::mozjs::jsapi::JSPropertySpec] = &[
			#(#specs),*
		];
	)
}

pub(crate) fn to_js_val(name: Ident) -> ItemImpl {
	let krate = quote!(::ion);
	parse_quote!(
		impl ::mozjs::conversions::ToJSValConvertible for #name {
			unsafe fn to_jsval(&self, cx: #krate::Context, mut rval: ::mozjs::rust::MutableHandleValue) {
				rval.set(<#name as #krate::ClassInitialiser>::new_object(cx, self.clone()).to_value());
			}
		}
	)
}

pub(crate) fn into_js_val(name: Ident) -> ItemImpl {
	let krate = quote!(::ion);
	parse_quote!(
		impl #krate::conversions::IntoJSVal for #name {
			unsafe fn into_jsval(self: Box<Self>, cx: #krate::Context, mut rval: ::mozjs::rust::MutableHandleValue) {
				rval.set(<#name as #krate::ClassInitialiser>::new_object(cx, *self).to_value());
			}
		}
	)
}

pub(crate) fn class_initialiser(name: Ident, constructor: &Method) -> ItemImpl {
	let krate = quote!(::ion);
	let ident = constructor.method.sig.ident.clone();
	let nargs = constructor.nargs as u32;
	let name_str = LitStr::new(&name.to_string(), ident.span());
	parse_quote!(
		impl #krate::ClassInitialiser for #name {
			const NAME: &'static str = #name_str;

			fn class() -> &'static ::mozjs::jsapi::JSClass {
				&CLASS
			}

			fn constructor() -> (::ion::NativeFunction, ::core::primitive::u32) {
				(#ident, #nargs)
			}

			fn functions() -> &'static [::mozjs::jsapi::JSFunctionSpec] {
				&FUNCTIONS
			}

			fn properties() -> &'static [::mozjs::jsapi::JSPropertySpec] {
				&PROPERTIES
			}

			fn static_functions() -> &'static [::mozjs::jsapi::JSFunctionSpec] {
				&STATIC_FUNCTIONS
			}

			fn static_properties() -> &'static [::mozjs::jsapi::JSPropertySpec] {
				&STATIC_PROPERTIES
			}
		}
	)
}
