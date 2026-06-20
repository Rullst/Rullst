class ApiController < ApplicationController
  def text
    render plain: 'Hello World'
  end

  def json_resp
    render json: { message: 'Hello World' }
  end

  def db_single
    record = Record.first || Record.new(name: 'Default')
    render json: { id: record.id, name: record.name }
  end

  def html_resp
    render html: '<h1>Hello World</h1>'.html_safe
  end
end
