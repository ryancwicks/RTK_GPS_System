FROM alpine:latest

RUN apk add --no-cache \
		bzip2 file gzip libffi libffi-dev krb5 krb5-dev \
		krb5-libs musl-dev openssh openssl-dev python3-dev \
		py3-cffi py3-cryptography py3-setuptools sshpass tar \
		rsync && \
	apk add --no-cache --virtual build-dependencies \
		gcc make && \
	python3 -m ensurepip --upgrade \
	  && \
	pip3 install ansible \
		&& \
	apk del build-dependencies \
		&& \
	rm -rf /root/.cache

RUN ansible-galaxy collection install community.docker

RUN mkdir /tmp/ansible
WORKDIR /tmp/ansible

ENTRYPOINT ["sh"]