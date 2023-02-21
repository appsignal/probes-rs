Vagrant.configure("2") do |config|
  config.vm.define "cgroups_v1" do |v1|
    v1.vm.box = "bento/ubuntu-20.04"
  end

  config.vm.define "cgroups_v2" do |v1|
    v1.vm.box = "bento/ubuntu-22.04"
  end

  config.vm.provision "shell", inline: <<-EOF.chomp
    sudo apt-get update

    sudo apt-get -y install \
      ca-certificates \
      curl \
      gnupg \
      lsb-release \
      make
    
    sudo mkdir -m 0755 -p /etc/apt/keyrings

    curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg

    echo \
    "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu \
    $(lsb_release -cs) stable" | sudo tee /etc/apt/sources.list.d/docker.list > /dev/null

    sudo apt-get update

    sudo chmod a+r /etc/apt/keyrings/docker.gpg

    sudo apt-get update

    sudo apt-get -y install docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin

    # Test it went well
    sudo docker run hello-world
    sudo docker info
  EOF
end
