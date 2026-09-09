fn main() {
    let mut owner =
        meld_dependency_security_owner::dependency_security::owner::SecurityPackageOwner::default();
    if let Err(error) = meld::runtime::owners::server::serve_stdio(&mut owner) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
