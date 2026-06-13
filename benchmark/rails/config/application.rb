require "rails"
require "action_controller/railtie"

Bundler.require(*Rails.groups)

module BenchRails
  class Application < Rails::Application
    config.load_defaults 7.1
    config.api_only = true
    config.logger = Logger.new(nil)
    config.log_level = :fatal
  end
end
