package storage

func migrate(db *DB, stmt *Stmt, tx *Tx, m string) error {
	if _, err := db.conn.Exec(m); err != nil {
		return err
	}
	if _, err := db.Exec("SELECT 1"); err != nil {
		return err
	}
	if _, err := stmt.Exec(1); err != nil {
		return err
	}
	_, err := tx.Exec("DELETE FROM t")
	return err
}
