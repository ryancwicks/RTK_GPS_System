"use strict"

export { build_page }

import { api_instance } from './gps_control_api.js';

//variable defines
let api = api_instance();

async function build_page() {

    let container = document.getElementById("status_div");

    const row_pairs = { 
        //"Field Name" : ["data field id", setting type]
        "GPS Fix": ["gps_fix_id", "basic_setting"],
        "Time": ["time_id", "basic_setting"],
        "Latitude (degrees)": ["latitude_id", "basic_setting"],
        "Longitude (degrees)": ["longitude_id", "basic_setting"],
        "Altitude (m)": ["altitude_id", "basic_setting"],
        "Speed (m/s)": ["speed_id", "basic_setting"],
    };

    let table = document.createElement("table");
    container.appendChild(table);
    for (var key of Object.keys(row_pairs)) {
        let row = document.createElement("tr");
        table.appendChild(row);

        let name_field = document.createElement("td");
        name_field.innerHTML = "<h3>"+ key +"</h3>";
        row.appendChild(name_field);

        let value_field = document.createElement("td");
        value_field.id = row_pairs[key][0];
        value_field.classList.add(row_pairs[key][1]);
        row.appendChild(value_field); 
    }

    let shutdown_link = document.createElement("A");
    shutdown_link.href = api.shutdown;
    shutdown_link.innerHTML = "Shutdown";
    container.appendChild(shutdown_link);

    //Setup the GPS data socket stream
    let socket = api.subscribe();
    //console.log(socket);

    socket.addEventListener('open', function (event) {
        //console.log(`Socket connected`);
    });

    socket.addEventListener('error', function (event) {
        console.error(`Websocket Error ${event.data}`);
    });

    socket.addEventListener('close', function (event) {
        //console.log(`Websocket Closed`);
    });
    socket.addEventListener('message', function (event) {
        var msg = JSON.parse(event.data);
        //console.log(msg);
        if (msg.mode) {
            document.getElementById("gps_fix_id").innerHTML = msg.mode;
        }
        if (msg.time) {
            document.getElementById("time_id").innerHTML = msg.time;
        }
        if (msg.lat) {
            document.getElementById("latitude_id").innerHTML = msg.lat;
        }
        if (msg.lon) {
            document.getElementById("longitude_id").innerHTML = msg.lon;
        }
        if (msg.alt) {
            document.getElementById("altitude_id").innerHTML = msg.alt;
        }
        if (msg.speed) {
            document.getElementById("speed_id").innerHTML = msg.speed;
        }
    });


};