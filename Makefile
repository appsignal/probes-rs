build:
	docker build -t probes/centos_7 docker/centos_7
	docker build -t probes/ubuntu_1204 docker/ubuntu_1204
	docker build -t probes/ubuntu_1404 docker/ubuntu_1404
	docker build -t probes/ubuntu_1604 docker/ubuntu_1604
	docker build -t probes/ubuntu_1804 docker/ubuntu_1804

test:
	docker run --rm \
		-v $(PWD)/tmp/centos_7/.cargo/registry:/root/.cargo/registry \
		-v $(PWD):/probes -t probes/centos_7 \
		/bin/bash -c "source /root/.cargo/env; cd /probes; rm -f Cargo.lock; cargo test"
	docker run --rm \
		-v $(PWD)/tmp/ubuntu_1204/.cargo/registry:/root/.cargo/registry \
		-v $(PWD):/probes -t probes/ubuntu_1404 \
		/bin/bash -c "source /root/.cargo/env; cd /probes; rm -f Cargo.lock; cargo test"
	docker run --rm \
		-v $(PWD)/tmp/ubuntu_1404/.cargo/registry:/root/.cargo/registry \
		-v $(PWD):/probes -t probes/ubuntu_1404 \
		/bin/bash -c "source /root/.cargo/env; cd /probes; rm -f Cargo.lock; cargo test"
	docker run --rm \
		-v $(PWD)/tmp/ubuntu_1604/.cargo/registry:/root/.cargo/registry \
		-v $(PWD):/probes -t probes/ubuntu_1604 \
		/bin/bash -c "source /root/.cargo/env; cd /probes; rm -f Cargo.lock; cargo test"
	docker run --rm \
		-v $(PWD)/tmp/ubuntu_1804/.cargo/registry:/root/.cargo/registry \
		-v $(PWD):/probes -t probes/ubuntu_1804 \
		/bin/bash -c "source /root/.cargo/env; cd /probes; rm -f Cargo.lock; cargo test"
