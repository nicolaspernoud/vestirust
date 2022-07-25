import 'dart:io';

import 'package:flutter/material.dart';
import 'package:webview_flutter/webview_flutter.dart';

class AppWebView extends StatefulWidget {
  @override
  AppWebViewState createState() => AppWebViewState();
}

class AppWebViewState extends State<AppWebView> {
  @override
  void initState() {
    super.initState();
    // Enable virtual display.
    if (Platform.isAndroid) WebView.platform = AndroidWebView();
  }

  @override
  Widget build(BuildContext context) {
    return WebView(
      initialUrl: 'http://app1.vestibule.127.0.0.1.nip.io:8080',
      javascriptMode: JavascriptMode.unrestricted,
    );
  }
}
