// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(conservative_impl_trait)]

use std::fmt::Debug;

fn foo<'a>(x: &'a i32) -> impl Debug + 'a {
    5
}

fn foo_not_static() -> impl Debug + 'static {
    let mut x = 5;
    x += 5;
    foo(&x)
    //~^ ERROR cannot infer an appropriate lifetime
}

fn main() {}