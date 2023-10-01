# Ansible docker container      

To set up this container, you need to add a password environment file in this directory:

```
echo 'VAULT_PASSWORD=<My vault password>' > vault_password.env
```

Then you can use docker compose to build and run ansible:

```
docker-compose build 
docker-compose run ansible
```

