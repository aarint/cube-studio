import { createClient } from 'redis';
import { connectDBDone, connectDBError } from '../actions/Connect';
import { addConnectedInstance, deleteInstance } from "../../utils/InstanceUtil";

export function testConnectDB(config) {
    return async () => {
        const client = createClient({
            socket: {
                host: config.ip,
                port: config.port,
            },
            password: config.password,
        });

        try {
            await client.connect();
        } finally {
            // Ensure we don't leave open sockets
            try {
                await client.quit();
            } catch (e) {
                // ignore
            }
        }
    };
}

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
            addConnectedInstance(config, client);
            dispatch(connectDBDone(config));
            return client;
        } catch (error) {
            console.error('Redis connection error:', error);
            dispatch(connectDBError(error));
            throw error;
        }
    }
}

export function disConnectDB() {

}