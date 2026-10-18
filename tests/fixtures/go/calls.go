package calls

import (
	"context"
	"os/exec"
)

func Run(ctx context.Context, db *DB, tx *Tx) {
	exec.Command("az", "account", "show").Run()
	exec.CommandContext(ctx, "aws", "configure", "export-credentials").Run()
	db.conn.Exec("SELECT 1")
	tx.Exec("DELETE FROM t")
}
