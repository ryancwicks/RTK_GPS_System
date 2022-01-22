#!/bin/sh
rm -rf /var/run/chrony
chronyd -d -f /etc/chrony/chrony.conf
