/*
 * The browser halves of a Web Authentication ceremony.
 *
 * Served as it is by DefaultResourcesFilter, so it has no build step and no
 * dependency. The default login page calls setupLogin, and the default
 * registration page calls setupRegistration; both talk to the endpoints of
 * WebAuthnConfigurer and nothing else.
 */
(function () {
  "use strict";

  // The endpoints of the four ceremonies, named as the server names them.
  var PATHS = {
    authenticateOptions: "/webauthn/authenticate/options",
    authenticate: "/webauthn/authenticate",
    registerOptions: "/webauthn/register/options",
    register: "/webauthn/register"
  };

  function base64UrlToBytes(value) {
    var base64 = value.replace(/-/g, "+").replace(/_/g, "/");
    var remainder = base64.length % 4;
    var padding = remainder === 0 ? "" : "=".repeat(4 - remainder);
    var binary = atob(base64 + padding);
    var bytes = new Uint8Array(binary.length);
    for (var i = 0; i < binary.length; i += 1) {
      bytes[i] = binary.charCodeAt(i);
    }

    return bytes;
  }

  function bytesToBase64Url(value) {
    var bytes = new Uint8Array(value);
    var binary = "";
    for (var i = 0; i < bytes.length; i += 1) {
      binary += String.fromCharCode(bytes[i]);
    }

    return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
  }

  function postJson(url, body, headers) {
    var allHeaders = Object.assign({ "Content-Type": "application/json" }, headers || {});

    return fetch(url, {
      method: "POST",
      credentials: "same-origin",
      headers: allHeaders,
      body: JSON.stringify(body)
    }).then(function (response) {
      return response.text().then(function (text) {
        var json = {};
        if (text) {
          try {
            json = JSON.parse(text);
          } catch (ignored) {
            json = { error: text };
          }
        }
        if (!response.ok) {
          throw new Error(json.error || response.statusText);
        }

        return json;
      });
    });
  }

  function showError(message) {
    var alert = document.createElement("div");
    alert.className = "alert alert-danger";
    alert.setAttribute("role", "alert");
    alert.textContent = message || "The ceremony failed.";
    document.body.insertBefore(alert, document.body.firstChild);
  }

  function supportsPasskeys() {
    return typeof window.PublicKeyCredential !== "undefined"
      && typeof navigator.credentials !== "undefined";
  }

  function decodedOptions(options) {
    var decoded = Object.assign({}, options);
    decoded.challenge = base64UrlToBytes(options.challenge);
    if (options.allowCredentials) {
      decoded.allowCredentials = options.allowCredentials.map(function (credential) {
        return Object.assign({}, credential, { id: base64UrlToBytes(credential.id) });
      });
    }
    if (options.excludeCredentials) {
      decoded.excludeCredentials = options.excludeCredentials.map(function (credential) {
        return Object.assign({}, credential, { id: base64UrlToBytes(credential.id) });
      });
    }
    if (options.user) {
      decoded.user = Object.assign({}, options.user, { id: base64UrlToBytes(options.user.id) });
    }

    return decoded;
  }

  function signIn(userName, contextPath, headers) {
    return postJson(contextPath + PATHS.authenticateOptions, { username: userName }, headers)
      .then(function (options) {
        return navigator.credentials.get({ publicKey: decodedOptions(options) });
      })
      .then(function (credential) {
        if (!credential) {
          throw new Error("The authenticator returned no credential.");
        }
        var response = credential.response;
        var assertion = {
          id: credential.id,
          rawId: bytesToBase64Url(credential.rawId),
          type: credential.type,
          response: {
            clientDataJSON: bytesToBase64Url(response.clientDataJSON),
            authenticatorData: bytesToBase64Url(response.authenticatorData),
            signature: bytesToBase64Url(response.signature)
          }
        };
        if (response.userHandle) {
          assertion.response.userHandle = bytesToBase64Url(response.userHandle);
        }

        return postJson(contextPath + PATHS.authenticate, assertion, headers);
      });
  }

  function register(contextPath, headers) {
    return postJson(contextPath + PATHS.registerOptions, {}, headers)
      .then(function (options) {
        return navigator.credentials.create({ publicKey: decodedOptions(options) });
      })
      .then(function (credential) {
        if (!credential) {
          throw new Error("The authenticator returned no credential.");
        }
        var response = credential.response;
        var registration = {
          id: credential.id,
          rawId: bytesToBase64Url(credential.rawId),
          type: credential.type,
          response: {
            clientDataJSON: bytesToBase64Url(response.clientDataJSON),
            attestationObject: bytesToBase64Url(response.attestationObject)
          }
        };
        if (typeof response.getTransports === "function") {
          registration.response.transports = response.getTransports();
        }

        return postJson(contextPath + PATHS.register, registration, headers);
      });
  }

  function userNameOf(parameter) {
    var field = document.querySelector('input[name="' + parameter + '"]');

    return field ? field.value.trim() : "";
  }

  /*
   * Wires the passkey sign-in button of the default login page.
   *
   * `headers` are the CSRF headers the page was rendered with, `parameter` is
   * the name of the field that holds the user name, and `registerUrl` is where a
   * user without a passkey can register one.
   */
  window.setupLogin = function (headers, contextPath, parameter, registerUrl, button) {
    if (!button) {
      return;
    }
    if (!supportsPasskeys()) {
      button.hidden = true;
      return;
    }

    button.addEventListener("click", function (event) {
      event.preventDefault();
      var userName = userNameOf(parameter);
      if (!userName) {
        showError("Enter your user name, then sign in with a passkey.");
        return;
      }
      signIn(userName, contextPath, headers)
        .then(function () {
          window.location.assign(contextPath + "/");
        })
        .catch(function (error) {
          showError(error.message);
        });
    });

    if (registerUrl) {
      var link = document.createElement("a");
      link.href = registerUrl;
      link.textContent = "Register a passkey";
      button.parentNode.appendChild(link);
    }
  };

  /*
   * Wires the passkey registration button of the default registration page.
   */
  window.setupRegistration = function (headers, contextPath, button) {
    if (!button) {
      return;
    }
    if (!supportsPasskeys()) {
      button.hidden = true;
      showError("This browser cannot register a passkey.");
      return;
    }

    button.addEventListener("click", function (event) {
      event.preventDefault();
      register(contextPath, headers)
        .then(function () {
          window.location.assign(contextPath + "/");
        })
        .catch(function (error) {
          showError(error.message);
        });
    });
  };
})();
