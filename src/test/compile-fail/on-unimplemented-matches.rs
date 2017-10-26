// Copyright 2017 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
#![feature(on_unimplemented)]
#![feature(attr_literals)]

struct MyType;

trait ImpldByMyType {}
trait NotImpldByMyType {}

impl ImpldByMyType for MyType {}

#[rustc_on_unimplemented(
    on(matches(bound="ImpldByMyType", selftype="MyType")), message="expectedmessage"
)]
trait ExpectedTrait {}

fn expect_trait<T: ExpectedTrait>(_: T) {}

fn main() {
    expect_trait(MyType);
    //~^ ERROR expectedmessage
}
