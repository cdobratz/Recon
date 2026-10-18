@app.get("/items")
def list_items():
    return []


@router.post("/items")
def create_item():
    return {}


urlpatterns = [
    path("/admin", admin),
]
