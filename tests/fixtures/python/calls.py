import subprocess
import os


def run_tool():
    subprocess.run(["ls"])
    os.system("ls")


def query(db):
    db.Exec("SELECT 1")
    db.conn.Exec("SELECT 2")
