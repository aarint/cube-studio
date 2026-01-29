import { createClient } from 'redis';
import { connectDBDone, connectDBError } from '../actions/Connect';
import { addConnectedInstance, deleteInstance } from "../../utils/InstanceUtil";

export function connectDB(config) {
    return async dispatch => {
        try {
            const client = createClient({
                socket: {
                    host: config.ip,
                    port: config.port,
                },
                password: config.password,
            });

            client.on('error', (err) => {
                console.error('Redis Client Error:', err);
                dispatch(connectDBError(err));
            });

            client.on('end', () => {
                console.log('Client :: end');
            });

            await client.connect();
            resolve(client);
            addConnectedInstance(config, client);
            dispatch(connectDBDone(config));
        } catch (error) {
            console.error('Redis connection error:', error);
            dispatch(connectDBError(error));
            throw error;
        }
    }
}

export function disConnectDB() {

}