# frozen_string_literal: true

require_relative 'boot'

require 'rails/all'
require 'good_job/engine'

# Require the gems listed in Gemfile, including any gems
# you've limited to :test, :development, or :production.
Bundler.require(*Rails.groups)

module Player
  class Application < Rails::Application
    # Initialize configuration defaults for originally generated Rails version.
    config.load_defaults 7.0

    config.active_job.queue_adapter = :good_job
    config.good_job = {
      preserve_job_records: true,
      retry_on_unhandled_error: false,
      on_thread_error: ->(exception) { Raven.capture_exception(exception) },
      execution_mode: :async,
      queues: '*',
      max_threads: 5,
      poll_interval: 30,
      shutdown_timeout: 25,
      enable_cron: false,
      cron: {
        # example: {
        #         cron: '0 * * * *',
        #         class: 'ExampleJob'
        # },
      }
    }

    # Configuration for the application, engines, and railties goes here.
    #
    # These settings can be overridden in specific environments using the files
    # in config/environments, which are processed later.
    #
    # config.time_zone = "Central Time (US & Canada)"
    # config.eager_load_paths << Rails.root.join("extras")
  end
end
