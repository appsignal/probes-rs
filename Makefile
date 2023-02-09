build:
	docker build -t probes/centos_7 -f docker/centos_7/Dockerfile .
	docker build -t probes/centos_8 -f docker/centos_8/Dockerfile .
	docker build -t probes/fedora_31 -f docker/fedora_31/Dockerfile .
	docker build -t probes/ubuntu_1404 -f docker/ubuntu_1404/Dockerfile .
	docker build -t probes/ubuntu_1604 -f docker/ubuntu_1604/Dockerfile .
	docker build -t probes/ubuntu_1804 -f docker/ubuntu_1804/Dockerfile .
	docker build -t probes/ubuntu_2004 -f docker/ubuntu_2004/Dockerfile .
	docker build -t probes/ubuntu_2204 -f docker/ubuntu_2204/Dockerfile .

test:
	docker run --rm \
		-v $(PWD)/tmp/centos_7/.cargo/registry:/root/.cargo/registry \
		-v $(PWD):/probes -t probes/centos_7 \
		/bin/bash -c "source /root/.cargo/env; cd /probes; cargo test"
	docker run --rm \
		-v $(PWD)/tmp/centos_8/.cargo/registry:/root/.cargo/registry \
		-v $(PWD):/probes -t probes/centos_8 \
		/bin/bash -c "source /root/.cargo/env; cd /probes; cargo test"
	docker run --rm \
		-v $(PWD)/tmp/fedora_31/.cargo/registry:/root/.cargo/registry \
		-v $(PWD):/probes -t probes/fedora_31 \
		/bin/bash -c "source /root/.cargo/env; cd /probes; cargo test"
	docker run --rm \
		-v $(PWD)/tmp/ubuntu_1404/.cargo/registry:/root/.cargo/registry \
		-v $(PWD):/probes -t probes/ubuntu_1404 \
		/bin/bash -c "source /root/.cargo/env; cd /probes; cargo test"
	docker run --rm \
		-v $(PWD)/tmp/ubuntu_1604/.cargo/registry:/root/.cargo/registry \
		-v $(PWD):/probes -t probes/ubuntu_1604 \
		/bin/bash -c "source /root/.cargo/env; cd /probes; cargo test"
	docker run --rm \
		-v $(PWD)/tmp/ubuntu_1804/.cargo/registry:/root/.cargo/registry \
		-v $(PWD):/probes -t probes/ubuntu_1804 \
		/bin/bash -c "source /root/.cargo/env; cd /probes; cargo test"
	docker run --rm \
		-v $(PWD)/tmp/ubuntu_2004/.cargo/registry:/root/.cargo/registry \
		-v $(PWD):/probes -t probes/ubuntu_2004 \
		/bin/bash -c "source /root/.cargo/env; cd /probes; cargo test"
	docker run --rm \
		-v $(PWD)/tmp/ubuntu_2204/.cargo/registry:/root/.cargo/registry \
		-v $(PWD):/probes -t probes/ubuntu_2204 \
		/bin/bash -c "source /root/.cargo/env; cd /probes; cargo test"
