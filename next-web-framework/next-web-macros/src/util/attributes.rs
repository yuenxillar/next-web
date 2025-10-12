use syn::ItemFn;

pub fn add_args<A>(item_fn: &mut ItemFn, args: A)
where
    A: Iterator<Item = proc_macro2::TokenStream>,
{
    for (index, arg) in args.enumerate() {
        // println!("adding arg: {}", arg.to_string());
        item_fn.sig.inputs.insert(index, syn::parse2(arg).unwrap());
    }
}
