"use strict"

export { api_instance }

function api_instance() {

    let requests = {
        socket_subscribe: "/api/subscribe",
        start_rtk: "/api/start_rtk",
        shutdown: "/api/shutdown",
    };


    /**
     * @brief Subscribe this client to the data streaming web socket.
     * @return web socket to listen on.
     */
     function subscribe( on_new_data_callback ) {
        let socket = new WebSocket("ws://" + window.location.host + requests.socket_subscribe);
        
        return socket;
    }

    async function start_rtk(username, password, server, mount_point, port) {
        let request_data = formRequest({
            "username": username,
            "password": password,
            "server": server,
            "mount_point": mount_point,
            "port": port,
        });

        let request;
        try{
            let request = await fetch(requests.start_rtk, request_data);
        } catch (e) {
            console.log(e);
            return false;
        }
        return true;
    }

    return {
        subscribe: subscribe,
        start_rtk: start_rtk,
        shutdown: requests.shutdown,
    };

    /**
     * @brief Method to simplify creation of fetch POST requests
     * @param {object} data object 
     * @return {object} Returns a fetch post request object.
     */
     function formRequest (data_object, method="POST") {
        const fetch_request = {
            method: method, 
            mode: 'cors', 
            cache: 'no-cache', 
            credentials: 'same-origin', 
            headers: {
              'Content-Type': 'application/json',
            },
            redirect: 'follow', 
            referrerPolicy: 'no-referrer', 
            body: JSON.stringify(data_object) 
          };

        return fetch_request;
    }
}