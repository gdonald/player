// Runs in the home lab's Jenkins. test.sh runs in a pod with the CI image and
// Postgres 17 as a second container, pinned to praxis, which holds the build
// cache under /home/k8s/player-ci-cache. When this passes, Jenkins reports the
// continuous-integration/jenkins/branch status, which starts the GitHub
// workflow.

def ciPod = '''
  apiVersion: v1
  kind: Pod
  spec:
    nodeSelector:
      kubernetes.io/hostname: praxis
    imagePullSecrets:
      - name: harbor-player
    containers:
      - name: ci
        image: harbor.home.gregdonald.com/player/ci:latest
        imagePullPolicy: Always
        command: ["sleep"]
        args: ["infinity"]
        securityContext:
          runAsUser: 1000
          runAsGroup: 1000
        env:
          - name: CARGO_HOME
            value: /cache/cargo
          - name: HOME
            value: /cache/home
          - name: npm_config_cache
            value: /cache/npm
          - name: PLAYWRIGHT_BROWSERS_PATH
            value: /cache/ms-playwright
          - name: PGHOST
            value: 127.0.0.1
          - name: PGUSER
            value: postgres
          - name: DATABASE_URL
            value: postgres://postgres@127.0.0.1/player_test
          - name: E2E_DATABASE_URL
            value: postgres://postgres@127.0.0.1/player_e2e
          - name: BROWSER_WORKERS
            value: "4"
        resources:
          requests:
            cpu: "4"
            memory: 6Gi
          limits:
            cpu: "8"
            memory: 8Gi
        volumeMounts:
          - name: cache
            mountPath: /cache
      - name: postgres
        image: postgres:17.11-trixie
        env:
          - name: POSTGRES_HOST_AUTH_METHOD
            value: trust
        resources:
          requests:
            memory: 256Mi
    volumes:
      - name: cache
        hostPath:
          path: /home/k8s/player-ci-cache
          type: Directory
'''

pipeline {
  agent {
    kubernetes {
      defaultContainer 'ci'
      yaml ciPod
    }
  }

  options {
    disableConcurrentBuilds(abortPrevious: true)
  }

  stages {
    stage('test') {
      steps {
        sh 'until pg_isready -q; do sleep 1; done'
        sh 'mkdir -p "$HOME"'
        sh 'cd features && npm ci && npx playwright install chromium'
        sh './test.sh'
      }
    }
  }

  post {
    always {
      archiveArtifacts artifacts: 'target/coverage/**, features/playwright-report/**', allowEmptyArchive: true
    }
  }
}
