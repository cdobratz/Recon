fn routes() {
    app.route("/items", get(list_items));
    web::resource("/health");
}
