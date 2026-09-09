fn main() {
    let mut owner = meld_docs_owner::docs::owner::DocsPackageOwner::default();
    if let Err(error) = meld::runtime::owners::server::serve_stdio(&mut owner) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
