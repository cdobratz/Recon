package sample

import "net/http"

func routes() {
	http.HandleFunc("/items", listItems)
	router.GET("/items", createItem)
}
