fn run(db: &Db) {
    std::process::Command::new("az");
    Command::new("terraform");
    db.exec("SELECT 1");
}
