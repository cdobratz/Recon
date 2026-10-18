package main

import (
	"context"
	"os/exec"
)

func run(ctx context.Context) {
	exec.Command("az", "account", "show").Run()
	exec.CommandContext(ctx, "aws", "configure", "export-credentials", "--format", "env").Run()
}
