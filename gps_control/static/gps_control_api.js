"use strict"

export { api_instance }

function api_instance() {

    let requests = {
        socket_subscribe: "/api/subscribe",
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

    return {
        subscribe: subscribe,
        shutdown: requests.shutdown,
    };
}