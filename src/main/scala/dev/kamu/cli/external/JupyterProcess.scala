package dev.kamu.cli.external

import java.awt.Desktop
import java.net.URI

import org.apache.hadoop.fs.FileSystem
import org.apache.log4j.LogManager

import scala.sys.process.{Process, ProcessIO}

class JupyterProcess(
  fileSystem: FileSystem,
  network: String,
  image: String = "kamu/jupyter:0.0.1"
) {
  protected val logger = LogManager.getLogger(getClass.getName)

  val containerName = "kamu-jupyter"
  var port: Long = 0
  var token: String = ""
  val dockerClient = new DockerClient()

  def run(): Process = {
    val propagateEnv = Seq("MAPBOX_ACCESS_TOKEN")

    val dockerArgs = Seq(
      "--network",
      network,
      "-v",
      s"${fileSystem.getWorkingDirectory.toUri.getPath}:/opt/workdir",
      "-P",
      "--name",
      containerName
    ) ++ propagateEnv
      .filter(e => sys.env.contains(e))
      .flatMap(e => Seq("-e", s"$e=${sys.env(e)}"))

    val cmd = dockerClient.makeRunCmd(image = image, extraArgs = dockerArgs)
    val processBuilder = dockerClient.prepare(cmd)
    processBuilder.run(ioHandler)
  }

  def ioHandler: ProcessIO = {
    val tokenRegex = raw"token=([a-z0-9]+)".r

    new ProcessIO(
      _ => (),
      stdout =>
        scala.io.Source
          .fromInputStream(stdout)
          .getLines()
          .foreach(line => System.out.println("[jupyter] " + line)),
      stderr =>
        scala.io.Source
          .fromInputStream(stderr)
          .getLines()
          .foreach(line => {
            synchronized {
              if (token.isEmpty) {
                token = tokenRegex
                  .findFirstMatchIn(line)
                  .map(m => m.group(1))
                  .getOrElse("")
                if (token.nonEmpty) {
                  logger.debug(s"Got Jupyter token: $token")
                  this.notifyAll()
                }
              }
            }
            System.err.println("[jupyter] " + line)
          })
    )
  }

  def openBrowserWhenReady(): Unit = {
    if (Desktop.isDesktopSupported && Desktop.getDesktop.isSupported(
          Desktop.Action.BROWSE
        )) {
      val pi = this

      val browserOpenerThread = new Thread {
        override def run(): Unit = {
          pi.synchronized {
            while (pi.token.isEmpty) {
              pi.wait()
            }
          }
          val containerPort =
            dockerClient.inspectHostPort(containerName, "80/tcp")
          val uri = URI.create(s"http://localhost:$containerPort/?token=$token")

          logger.info(s"Opening in browser: $uri")
          Desktop.getDesktop.browse(uri)
        }
      }

      browserOpenerThread.setDaemon(true)
      browserOpenerThread.start()
    }
  }

  def stop(): Unit = {
    dockerClient.kill(containerName)
  }

  // TODO: avoid this by setting up correct user inside the container
  def chown(): Unit = {
    logger.debug("Fixing file ownership")

    val dockerArgs = Seq(
      "-v",
      s"${fileSystem.getWorkingDirectory.toUri.getPath}:/opt/workdir"
    )

    val unix = new com.sun.security.auth.module.UnixSystem()
    val shellCommand = Seq(
      "chown",
      "-R",
      s"${unix.getUid}:${unix.getGid}",
      "/opt/workdir"
    )

    dockerClient.runShell(
      image = image,
      shellCommand = shellCommand,
      extraArgs = dockerArgs
    )
  }
}