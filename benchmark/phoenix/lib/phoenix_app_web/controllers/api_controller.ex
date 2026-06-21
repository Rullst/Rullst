defmodule PhoenixAppWeb.ApiController do
  use PhoenixAppWeb, :controller
  alias PhoenixApp.Repo
  alias PhoenixApp.Record

  def text(conn, _params) do
    text(conn, "Hello World")
  end

  def json_resp(conn, _params) do
    json(conn, %{message: "Hello World"})
  end

  def db_single(conn, _params) do
    record = Repo.get(Record, 1) || %Record{name: "Default"}
    json(conn, %{id: record.id, name: record.name})
  end

  def html_resp(conn, _params) do
    html(conn, "<h1>Hello World</h1>")
  end
end
