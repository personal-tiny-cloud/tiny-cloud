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

function setErrorMsg(string) {
	var error_msg = document.getElementById("errormsg");
	error_msg.innerHTML = string;
}

async function submit(form) {
	var formData = new FormData(form);
	let response = await fetch(prefix + 'api/auth/login', {
		method: "POST",
		mode: "same-origin",
		cache: "no-cache",
		credentials: "same-origin",
		headers: {
			"Content-Type": "application/json",
		},
		redirect: "follow",
		referrerPolicy: "no-referrer",
		body: JSON.stringify(Object.fromEntries(formData)),
	});

	if (response.status !== 200) {
		let errInfo = await response.json();
		switch (errInfo.error) {
			case 'BadCredentials':
				setErrorMsg(errInfo.message + '<br>Check user and password and try again.');
				break;
			case 'InternalError':
				setErrorMsg(errInfo.message + '<br>Internal server error occurred... Check server logs if this persits');
				break;
			case 'InvalidCredentials':
				setErrorMsg(errInfo.message + '<br>Invalid credentials. Cannot login');
				break;
			default:
				setErrorMsg('Unexpected error... This may be a bug, check logs and open an issue if this persists');
				console.log(errInfo);
				break;
		}
	} else {
		window.location.reload();
	}
}

window.onload = function() {
	var login = document.getElementById('login');
	login.onsubmit = function(event) {
		event.preventDefault();
		try {
			submit(login);
		} catch (error) {
			setErrorMsg('A JS error occurred, check logs for more info and open an issue if this persists');
			console.log(error);
		}
		return false;
	};
}

