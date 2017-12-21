import React from 'react';
import { Button, Layout } from 'antd';

import Instances from './Instances';

var os = window.require('os');
console.log("got:", os.cpus());

//const { dialog } = window.require('electron');
const { dialog } = window.require('electron').remote;
const { Content, Sider } = Layout;

export default class Main extends React.PureComponent {


    openConnection ()  {
        console.log('shit');
        console.log(dialog);

        dialog.showOpenDialog({ properties: ['openFile', 'openDirectory', 'multiSelections'] })
    }

    render() {
        return (
            <Layout>
                <Sider style={styles.sider}>
                    <div>shit sider</div>
                </Sider>
                <Content style={{ marginLeft: '200px', overflow: 'initial' }}>
                    <div style={{ padding: 0, background: '#fff', textAlign: 'center' }}>
                        <div>{os.cpus().length}</div>
                        <Button type="primary" onClick={this.openConnection}>primary</Button>
                        <Instances />
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