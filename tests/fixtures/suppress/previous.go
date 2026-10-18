package main

import "os/exec"

func run() {
	// recon:ignore recon/shell-execution
	exec.Command("az", "account", "show").Run()
	exec.Command("aws").Run()
}

func runBlock() {
	/* recon:ignore recon/shell-execution */
	exec.Command("az", "group", "list").Run()
}
