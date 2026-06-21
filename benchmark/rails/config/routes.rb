Rails.application.routes.draw do
  get '/text', to: 'api#text'
  get '/json', to: 'api#json_resp'
  get '/db-single', to: 'api#db_single'
  get '/html', to: 'api#html_resp'
end
