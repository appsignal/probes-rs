build:
	docker build -t probes/centos_7 docker/centos_7
	docker build -t probes/ubuntu_1204 docker/ubuntu_1204
	docker build -t probes/ubuntu_1404 docker/ubuntu_1404
	docker build -t probes/ubuntu_1604 docker/ubuntu_1604

test:
	docker run -v $(PWD):/probes -t probes/centos_7 /bin/bash -c "source /root/.cargo/env; cd /probes; rm -f Cargo.lock; cargo test"
	docker run -v $(PWD):/probes -t probes/ubuntu_1204 /bin/bash -c "source /root/.cargo/env; cd /probes; rm -f Cargo.lock; cargo test"
	docker run -v $(PWD):/probes -t probes/ubuntu_1404 /bin/bash -c "source /root/.cargo/env; cd /probes; rm -f Cargo.lock; cargo test"
	docker run -v $(PWD):/probes -t probes/ubuntu_1604 /bin/bash -c "source /root/.cargo/env; cd /probes; rm -f Cargo.lock; cargo test"
