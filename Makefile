build:
	docker build -t probes/ubuntu_2204 -f docker/ubuntu_2204/Dockerfile .

build-all:
	docker build -t probes/centos_7 -f docker/centos_7/Dockerfile .
	docker build -t probes/centos_8 -f docker/centos_8/Dockerfile .
	docker build -t probes/fedora_31 -f docker/fedora_31/Dockerfile .
	docker build -t probes/ubuntu_1404 -f docker/ubuntu_1404/Dockerfile .
	docker build -t probes/ubuntu_1604 -f docker/ubuntu_1604/Dockerfile .
	docker build -t probes/ubuntu_1804 -f docker/ubuntu_1804/Dockerfile .
	docker build -t probes/ubuntu_2004 -f docker/ubuntu_2004/Dockerfile .
	docker build -t probes/ubuntu_2204 -f docker/ubuntu_2204/Dockerfile .

check:
	docker run --rm \
		-v $(PWD)/tmp/.cargo/registry/cache/ubuntu_2204:/root/.cargo/registry/cache \
		-v $(PWD)/tmp/.cargo/registry/index:/root/.cargo/registry/index \
		-v $(PWD)/tmp/.cargo/registry/src:/root/.cargo/registry/src \
		-v $(PWD):/probes -t probes/ubuntu_2204 \
		/bin/bash -c "source /root/.cargo/env; cd /probes; cargo check"

test:
	docker run --rm \
		-v $(PWD)/tmp/.cargo/registry/cache/ubuntu_2204:/root/.cargo/registry/cache \
		-v $(PWD)/tmp/.cargo/registry/index:/root/.cargo/registry/index \
		-v $(PWD)/tmp/.cargo/registry/src:/root/.cargo/registry/src \
		-v $(PWD):/probes -t probes/ubuntu_2204 \
		/bin/bash -c "source /root/.cargo/env; cd /probes; cargo test"

publish:
	docker run --rm \
		-v $(PWD)/tmp/.cargo/registry/cache/ubuntu_2204:/root/.cargo/registry/cache \
		-v $(PWD)/tmp/.cargo/registry/index:/root/.cargo/registry/index \
		-v $(PWD)/tmp/.cargo/registry/src:/root/.cargo/registry/src \
		-v $(PWD):/probes -it probes/ubuntu_2204 \
		/bin/bash -c "source /root/.cargo/env; cd /probes; cargo login; cargo publish"

test-all:
	docker run --rm \
		-v $(PWD)/tmp/.cargo/registry/cache/centos_7:/root/.cargo/registry/cache \
		-v $(PWD)/tmp/.cargo/registry/index:/root/.cargo/registry/index \
		-v $(PWD)/tmp/.cargo/registry/src:/root/.cargo/registry/src \
		-v $(PWD):/probes -t probes/centos_7 \
		/bin/bash -c "source /root/.cargo/env; cd /probes; cargo test"
	docker run --rm \
		-v $(PWD)/tmp/.cargo/registry/cache/centos_8:/root/.cargo/registry/cache \
		-v $(PWD)/tmp/.cargo/registry/index:/root/.cargo/registry/index \
		-v $(PWD)/tmp/.cargo/registry/src:/root/.cargo/registry/src \
		-v $(PWD):/probes -t probes/centos_8 \
		/bin/bash -c "source /root/.cargo/env; cd /probes; cargo test"
	docker run --rm \
		-v $(PWD)/tmp/.cargo/registry/cache/fedora_31:/root/.cargo/registry/cache \
		-v $(PWD)/tmp/.cargo/registry/index:/root/.cargo/registry/index \
		-v $(PWD)/tmp/.cargo/registry/src:/root/.cargo/registry/src \
		-v $(PWD):/probes -t probes/fedora_31 \
		/bin/bash -c "source /root/.cargo/env; cd /probes; cargo test"
	docker run --rm \
		-v $(PWD)/tmp/.cargo/registry/cache/ubuntu_1404:/root/.cargo/registry/cache \
		-v $(PWD)/tmp/.cargo/registry/index:/root/.cargo/registry/index \
		-v $(PWD)/tmp/.cargo/registry/src:/root/.cargo/registry/src \
		-v $(PWD):/probes -t probes/ubuntu_1404 \
		/bin/bash -c "source /root/.cargo/env; cd /probes; cargo test"
	docker run --rm \
		-v $(PWD)/tmp/.cargo/registry/cache/ubuntu_1604:/root/.cargo/registry/cache \
		-v $(PWD)/tmp/.cargo/registry/index:/root/.cargo/registry/index \
		-v $(PWD)/tmp/.cargo/registry/src:/root/.cargo/registry/src \
		-v $(PWD):/probes -t probes/ubuntu_1604 \
		/bin/bash -c "source /root/.cargo/env; cd /probes; cargo test"
	docker run --rm \
		-v $(PWD)/tmp/.cargo/registry/cache/ubuntu_1804:/root/.cargo/registry/cache \
		-v $(PWD)/tmp/.cargo/registry/index:/root/.cargo/registry/index \
		-v $(PWD)/tmp/.cargo/registry/src:/root/.cargo/registry/src \
		-v $(PWD):/probes -t probes/ubuntu_1804 \
		/bin/bash -c "source /root/.cargo/env; cd /probes; cargo test"
	docker run --rm \
		-v $(PWD)/tmp/.cargo/registry/cache/ubuntu_2004:/root/.cargo/registry/cache \
		-v $(PWD)/tmp/.cargo/registry/index:/root/.cargo/registry/index \
		-v $(PWD)/tmp/.cargo/registry/src:/root/.cargo/registry/src \
		-v $(PWD):/probes -t probes/ubuntu_2004 \
		/bin/bash -c "source /root/.cargo/env; cd /probes; cargo test"
	docker run --rm \
		-v $(PWD)/tmp/.cargo/registry/cache/ubuntu_2204:/root/.cargo/registry/cache \
		-v $(PWD)/tmp/.cargo/registry/index:/root/.cargo/registry/index \
		-v $(PWD)/tmp/.cargo/registry/src:/root/.cargo/registry/src \
		-v $(PWD):/probes -t probes/ubuntu_2204 \
		/bin/bash -c "source /root/.cargo/env; cd /probes; cargo test"

test-cgroups-v1:
	vagrant up cgroups_v1
	vagrant ssh cgroups_v1 -c "cd /vagrant; sudo -E make build test"

test-cgroups-v2:
	vagrant up cgroups_v2
	vagrant ssh cgroups_v2 -c "cd /vagrant; sudo -E make build test"

test-cgroups: test-cgroups-v1 test-cgroups-v2
	@echo ''
	@echo 'Done! If desired, run `vagrant halt` to stop the machines, or `vagrant destroy -f` to destroy them.'
