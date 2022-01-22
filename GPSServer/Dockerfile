FROM alpine:3.14

USER root

RUN apk add --no-cache gpsd 
RUN echo "pps_ldisc" >> /etc/modules
EXPOSE 2947

COPY startup_gpsd.sh /tmp/startup_gpsd.sh
WORKDIR /tmp

ENTRYPOINT ["./startup_gpsd.sh"]
