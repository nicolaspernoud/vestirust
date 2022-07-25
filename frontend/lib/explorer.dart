import 'dart:typed_data';

import 'package:dio/dio.dart';
import 'package:flutter/material.dart';
import 'package:webdav_client/webdav_client.dart' as webdav;

class Explorer extends StatefulWidget {
  const Explorer({Key? key}) : super(key: key);

  @override
  _ExplorerState createState() => _ExplorerState();
}

enum CopyMoveStatus { none, copy, cut }

class _ExplorerState extends State<Explorer> {
  late webdav.Client client;
  final url = 'http://files2.vestibule.10.0.2.2.nip.io:8080';
  final user = 'hello';
  final pwd = 'world';
  var dirPath = '/';
  var _copyMoveStatus = CopyMoveStatus.none;
  var _copyMovePath = "";

  @override
  void initState() {
    super.initState();

    // init client
    client = webdav.newClient(
      url,
      user: user,
      password: pwd,
      debug: true,
    );
  }

  @override
  Widget build(BuildContext context) {
    if (url.isEmpty || user.isEmpty || pwd.isEmpty) {
      return const Center(child: Text("you need add url || user || pwd"));
    }
    return Scaffold(
      appBar: AppBar(
        title: const Text('Webdav test'),
      ),
      body: FutureBuilder(
          future: _getData(),
          builder: (BuildContext context,
              AsyncSnapshot<List<webdav.File>> snapshot) {
            switch (snapshot.connectionState) {
              case ConnectionState.none:
              case ConnectionState.active:
              case ConnectionState.waiting:
                return const Center(child: CircularProgressIndicator());
              case ConnectionState.done:
                if (snapshot.hasError) {
                  return Center(child: Text('Error: ${snapshot.error}'));
                }
                return _buildListView(context, snapshot.data ?? []);
            }
          }),
      bottomNavigationBar: BottomAppBar(
          child: Row(children: [
        IconButton(
            icon: const Icon(Icons.home),
            onPressed: () {
              dirPath = "/";
              setState(() {
                _getData();
              });
            }),
        IconButton(
            icon: const Icon(Icons.create_new_folder),
            onPressed: () async {
              CancelToken c = CancelToken();
              await client.mkdir("$dirPath/newfolder", c);
              setState(() {
                _getData();
              });
            }),
        IconButton(
            icon: const Icon(Icons.add),
            onPressed: () async {
              CancelToken c = CancelToken();
              await client.write("$dirPath/newfile.txt", Uint8List(0),
                  onProgress: (c, t) {
                print(c / t);
              }, cancelToken: c);
              setState(() {
                _getData();
              });
            }),
        if (_copyMoveStatus != CopyMoveStatus.none)
          IconButton(
              icon: const Icon(Icons.paste),
              onPressed: () async {
                CancelToken c = CancelToken();
                await client.copy(_copyMovePath, dirPath, true, c);
                setState(() {
                  _getData();
                });
              })
      ])),
    );
  }

  Future<List<webdav.File>> _getData() {
    return client.readDir(dirPath);
  }

  Widget _buildListView(BuildContext context, List<webdav.File> list) {
    return ListView.builder(
        itemCount: list.length,
        itemBuilder: (context, index) {
          final file = list[index];
          return ListTile(
            leading: Icon(
                file.isDir == true ? Icons.folder : Icons.file_present_rounded),
            title: Text(file.name ?? ''),
            subtitle: Text(file.mTime.toString()),
            trailing: Row(mainAxisSize: MainAxisSize.min, children: [
              IconButton(
                  icon: const Icon(Icons.copy),
                  onPressed: (() {
                    setState(() {
                      _copyMoveStatus = CopyMoveStatus.copy;
                      _copyMovePath = file.path!;
                    });
                  })),
              IconButton(
                  icon: const Icon(Icons.delete),
                  onPressed: (() {
                    client.removeAll(file.path!);
                    setState(() {
                      _getData();
                    });
                  }))
            ]),
            onTap: () {
              if (file.isDir!) {
                dirPath = file.path!;
                setState(() {
                  _getData();
                });
              }
            },
          );
        });
  }
}
