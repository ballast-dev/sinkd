from invoke.tasks import task
from invoke.context import Context
from pathlib import Path
import os


def top_level_dir() -> Path:
    cwd = Path(os.getcwd())
    while cwd != cwd.root:
        if cwd.joinpath(".git").exists():
            return cwd
        cwd = cwd.parent
    print("fatal: unable to find .git in this or any parent directory")
    exit(1)


TLD = top_level_dir()


@task(default=True)
def img_build(c: Context):
    with c.cd(TLD.joinpath("docker")):
        c.run("docker build -t sinkd .", pty=True)


@task
def up(c: Context):
    with c.cd(TLD.joinpath("docker")):
        c.run(
            f"TLD={TLD} docker compose up --detach sinkd",
            pty=True,
        )


@task
def down(c: Context):
    with c.cd(TLD.joinpath("docker")):
        c.run(
            f"TLD={TLD} docker compose down sinkd",
            pty=True,
        )


@task
def run(c: Context):
    with c.cd(TLD.joinpath("docker")):
        c.run(f"TLD={TLD} docker compose exec -u sinkd sinkd bash", pty=True)
