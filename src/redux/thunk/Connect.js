import Redis from 'redis';
import { connectDBDone, connectDBError } from '../actions/Connect';
import { addConnectedInstance, deleteInstance } from "../../utils/InstanceUtil";

export function connectDB(config) {
    return dispatch => {
        return new Promise((resolve, reject) => {
            let client = Redis.createClient(config.port, config.ip);

            client.on('connect', function () {
                resolve(client);
                addConnectedInstance(config, client);
                dispatch(connectDBDone(config));
            })

            client.on('end', () => {
                console.log('Client :: end');
            });
        })
    }
}

export function disConnectDB() {

}