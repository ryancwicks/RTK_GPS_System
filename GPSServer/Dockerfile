FROM alpine:3.14

USER root

RUN apk add --no-cache gpsd 
RUN echo "pps_gpio" >> /etc/modules

COPY startup_gpsd.sh /tmp/startup_gpsd.sh
WORKDIR /tmp

EXPOSE 2947

ENTRYPOINT ["./startup_gpsd.sh"]
