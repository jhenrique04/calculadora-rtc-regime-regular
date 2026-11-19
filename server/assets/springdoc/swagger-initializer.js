window.onload = function() {
  //<editor-fold desc="Changeable Configuration Block">

  // the following lines will be replaced by docker/configurator, when it runs in a docker-container
  window.ui = SwaggerUIBundle({
    url: "",
    dom_id: '#swagger-ui',
    deepLinking: true,
    presets: [
      SwaggerUIBundle.presets.apis,
      SwaggerUIStandalonePreset
    ],
    plugins: [
      SwaggerUIBundle.plugins.DownloadUrl
    ],
    layout: "BaseLayout" ,

  "configUrl" : "/api/api-docs/swagger-config",
  "displayRequestDuration" : true,
  "layout" : "BaseLayout",
  "operationsSorter" : "method",
  "supportedSubmitMethods" : [ "get", "post" ],
  "validatorUrl" : ""

  });

  //</editor-fold>
};
