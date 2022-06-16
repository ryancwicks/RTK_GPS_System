FROM alpine:3.16

USER root

RUN apk add --no-cache gcc git make musl-dev

WORKDIR /opt

RUN git clone https://github.com/ryancwicks/RTKLIB.git; cd RTKLIB/app/str2str/gcc; sed '/^CFLAGS/ s/$/ -std=gnu99/' makefile > Makefile; rm makefile; make; cp str2str /opt; cd /opt; rm -rf RTKLIB

ENTRYPOINT ["sh"]
