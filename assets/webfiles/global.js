"use strict";
try {
	let _prefix = document.querySelector('meta[name="tcloud-prefix"]').content;
	if (_prefix == "") {
		var prefix = "/";
	} else {
		var prefix = "/" + _prefix + "/";
	}
} catch (e) {
	var prefix = "/";
}
