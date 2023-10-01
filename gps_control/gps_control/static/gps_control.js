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
        "RMS ": ["rms_id", "basic_setting"],
        "Major (m)": ["major_id", "basic_setting"],
        "Minor (m)": ["minor_id", "basic_setting"],
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

    //let rtk_div = create_rtk_client_section();
    //container.appendChild(rtk_div);

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
        if (msg.rms) {
            document.getElementById("rms_id").innerHTML = msg.rms;
        }
        if (msg.major) {
            document.getElementById("major_id").innerHTML = msg.major;
        }
        if (msg.minor) {
            document.getElementById("minor_id").innerHTML = msg.minor;
        }
    });
};

function create_rtk_client_section() {
    let rtk_div = document.createElement("div");

    let rtk_div_name = document.createElement("H2");
    rtk_div_name.innerHTML = "RTK Client Settings";
    rtk_div.appendChild(rtk_div_name);
    
    //input_table createion
    const row_pairs = { 
        //"Field Name" : ["data field id", input type, default_value]
        "Server": ["rtk_server_id", "text", "rtk2go.com"],
        "Port": ["rtk_port_id", "number", 2101],
        "Mount Point": ["rtk_mount_point_id", "text", "AVRIL"],
        "Username/E-mail": ["rtk_username_id", "text", "ryan@voyis.com"],
        "Password": ["rtk_password_id", "text", "none"],
    };

    let table = document.createElement("table");
    rtk_div.appendChild(table);
    for (var key of Object.keys(row_pairs)) {
        let row = document.createElement("tr");
        table.appendChild(row);

        let name_field = document.createElement("td");
        name_field.innerHTML = "<h3>"+ key +"</h3>";
        row.appendChild(name_field);

        let value_field = document.createElement("input");
        value_field.id = row_pairs[key][0];
        value_field.type = row_pairs[key][1];
        value_field.value = row_pairs[key][2];
        value_field.classList.add(row_pairs[key][1]);
        row.appendChild(value_field); 
    }

    
    let start_rtk_button = document.createElement("input");
    start_rtk_button.type = "button";
    start_rtk_button.id = "rtk_button_id";
    start_rtk_button.value = "Start RTK";
    start_rtk_button.addEventListener("click", async () => {
        let username = document.getElementById("rtk_username_id").value;
        let password = document.getElementById("rtk_password_id").value;
        let port = parseInt(document.getElementById("rtk_port_id").value);
        let server = document.getElementById("rtk_server_id").value;
        let mount_point = document.getElementById("rtk_mount_point_id").value;
        await api.start_rtk(username, password, server, mount_point, port);
    });
    rtk_div.appendChild(start_rtk_button);

    return rtk_div;
}