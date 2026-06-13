Rails.application.routes.draw do
  get "/", to: proc { [200, {"Content-Type" => "text/plain"}, ["Hello, World!"]] }
  get "/json", to: proc { [200, {"Content-Type" => "application/json"}, ['{"message":"Hello, World!"}']] }
end
