FROM alpine:3.14

USER root

RUN apk add --no-cache chrony

COPY chrony.conf /etc/chrony/chrony.conf
COPY startup_chronyd.sh /tmp/startup_chronyd.sh

WORKDIR /tmp

EXPOSE 323

ENTRYPOINT [ "./startup_chronyd.sh" ]
