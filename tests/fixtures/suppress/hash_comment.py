subprocess.run(["ls"])  # recon:ignore recon/shell-execution
# recon:ignore recon/shell-execution, recon/not-a-real-rule
os.system("ls")
