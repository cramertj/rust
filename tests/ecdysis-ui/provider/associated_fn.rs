#[provider(id = "wrapper_struct")]
type WrapperStruct;

fn main() {
    // FIXME(ecdysis): get this resolving properly.
    let x = WrapperStruct::<u64>::new(22);
    //~^ ERROR Unable to resolve provided item
    assert_eq!(x.field_name, 22);
}
