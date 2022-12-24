.PHONY: image
image:
	sudo docker build -t alpine .

.PHONY: container
container:
	sudo docker run --name sinkd -itd alpine	

.PHONY: build 
build:
	sudo docker exec sinkd cargo build
