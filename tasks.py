from invoke import Context, task


@task(default=True)
def img_build(c: Context):
    with c.cd("docker"):
        c.run("docker build -t sinkd .", pty=True)


@task
def run(c: Context):
    # with c.cd("docker"):
    #     c.run("docker compose up sinkd", pty=True)
    c.run("docker run -it --rm --hostname anchor -v .:/repo sinkd", pty=True)
