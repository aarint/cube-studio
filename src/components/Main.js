/**
 * Main page show the all connections that include saved in storage.
 * Click one will show the DB instance in the opened tab window.
 */

import React from 'react';
import { Button, Layout } from 'antd';
import Instance from './Instance';

const { Content, Sider } = Layout;

export default class Main extends React.PureComponent {

    /**
     * Open connection dialog. Connect the DB after input the connection config.
     * Then create a new tab that contain the DB instance.
     */
    openConnection() {
        //dialog.showOpenDialog({ properties: ['openFile', 'openDirectory', 'multiSelections'] })
    }

    render() {
        return (
            <Layout>
                <Content style={{ marginLeft: '200px', overflow: 'initial' }}>
                    <div style={{ padding: 0, background: '#fff', textAlign: 'center' }}>
                        <div></div>
                        <Button type="primary" onClick={this.openConnection}>primary</Button>
                    </div>
                </Content>
            </Layout>
        )
    }
}

const styles = {
    sider: {
        color: 'green',
        overflow: 'auto',
        height: '100vh',
        position: 'fixed',
        left: 0,
        border: 'solid 1 silver',
        background: "black"
    }
}