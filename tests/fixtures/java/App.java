public class App {
    @GetMapping("/items")
    public String items(String id) {
        return id;
    }
}
