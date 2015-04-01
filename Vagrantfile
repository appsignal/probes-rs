# -*- mode: ruby -*-
# vi: set ft=ruby :

Vagrant.configure(2) do |config|
  config.vm.box = "http://files.vagrantup.com/lucid64.box"

  config.vm.provision "shell", inline: <<-SHELL
    sudo apt-get update
    sudo apt-get upgrade -y
    sudo apt-get install build-essential curl -y
    curl -s https://static.rust-lang.org/rustup.sh | sudo sh
  SHELL
end
